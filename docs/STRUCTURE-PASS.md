# The structure pass

**Kind:** Reference
**Status:** Built: phases 0 to 7 and the lints that hold them.
**Decisions:** F70 (the key boundary), F71 (dependency policy), F72
(the hard fork) and F74 (the structure enforced).

The pass that made the repository easy to audit, and the rules it set
that hold now: the dependency policy (§6), the layout rule (§7.1), the
enforcement (§7.2) and the shape of the areas (§7.3).

## 1. What this changes in the plan

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

## 2. Phase 0: what is false and what is personal

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

## 3. Phase 0b: the hard fork (F72)

Built (F72). The order within the phase: moved to `docs/history/STRUCTURE-PASS-LOG.md`.

### 3.1 Names

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

### 3.2 What the rename must not touch

The rename edits crate names, paths and Rust identifiers (`osk_bip::`)
from an exact list of tokens. It never edits a pattern such as `osk-`,
because these byte strings contain one and are part of a format, a
derivation or a file a person already holds:

- **Magics:** `OSKB` (backup files), `OSKC` (threshold carry), `OSKO`
  (baked font outlines).
- **Extensions:** `.oskb`, `.osk` (`partly-signed.osk`).
- **Domain-separation and nonce tags:**
  - `osk-sealed-v1`, `osk-sealed-nonce`, `osk-session-key-weak`,
    `osk-secp-blind`;
  - `osk-schnorr-aux`;
  - `OpenSigner/musig-rand`, `OpenSigner/musig-next-seed`;
  - `OpenSigner/threshold-rand`, `OpenSigner/threshold-blind`;
  - `osk-kdbx-uuid`.
- **Text formats:** the `osk-threshold` and `osk-silent` headers.
- **Names of the external format in the strings:** ".oskb backup"
  and ".oskb". That is what the file is called.

What the rename was expected to change, and its check: moved to `docs/history/STRUCTURE-PASS-LOG.md`.

### 3.3 Provenance, frozen before the rename

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

### 3.4 Made Faraday's

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

## 4. Phases 1 and 2: maps and the docs layout

`tools/lint-docs.sh`, in `just lint`, checks the status header on every
doc (Kind is Reference, Spec or History) and that every path a doc names
resolves; it skips `docs/history/` for path resolution. Each crate's
tests are one binary, `tests/all/main.rs`, one module per former test
file, with the shared helpers in `common`.

faraday-intake has its own vector tests (phase 6).

What phases 1 and 2 did: moved to `docs/history/STRUCTURE-PASS-LOG.md`.

## 5. The key boundary (F70)

Built; the boundary is `docs/KEYS.md` (F70).

The plan as written: moved to `docs/history/STRUCTURE-PASS-LOG.md`.

## 6. Dependency policy (F71)

`cargo vet --locked` is in `just lint`
and passes: 88 trust entries, the imported audits, and 141 exemptions
(223 before). Trust is by the publisher of each locked version, all
under F71's named groups: rust-bitcoin (apoelstra: `bitcoin`,
`bitcoin-io`, `bitcoin-internals`, `bech32`, `miniscript`), RustCrypto
(tarcieri and RustCrypto's GitHub publishing, `github:RustCrypto/*`:
the hashes, MACs, AEADs, password hashes, `rsa`, `der`/`spki`/`pkcs*`,
`crypto-bigint`, `signature`, `ed25519`, `zeroize`), dalek (rozbb:
`curve25519-dalek`, `ed25519-dalek`), and the publishers Mozilla, ISRG
and the Bytecode Alliance already trust (dtolnay, sunfishcode,
kennykerr, BurntSushi, epage, alexcrichton, rust-lang-owner, cuviper).
Every trust entry ends on 2027-10-01; the board carries the renewal a
month before. Exempt and not reviewed where the publisher is in none of
those groups: `crypto-primes` (fjarri), `fiat-crypto` (JasonGross),
`hex_lit` (another rust-bitcoin maintainer, not named in F71), and the
desktop window stack's long tail. `just vet-refresh` (online) updates the
imports and prunes. Every direct dependency has a `docs/deps/` page with
a **Review.** line saying which of the three covers it. `third_party/rqrr`
carries `[policy.rqrr] audit-as-crates-io = false`: offline, cargo vet
took the vendored copy as first-party already, but `cargo vet` online
(as `vet-refresh` runs it) refuses it without the line, since it matches
a published version. No real audit was written (F71's sha1 and PGP case): `sha1` and
every crate `faraday-pgp` uses are now trusted by publisher; an audit by
reading remains possible, and would be labelled by who read it.

The measurements that led to F71: moved to `docs/history/STRUCTURE-PASS-LOG.md`.

## 7. Phases 3, 4, 6 and 7

Phase 3 (built: the unused functions deleted, the token backlog held by `tools/lint-tokens.counts`): moved to `docs/history/STRUCTURE-PASS-LOG.md`.

Built in phase 6:
- **The strings** (A11): `en.rs` is one
  `strings!` list that makes both `Strings` and `EN`, each key's doc
  above its value (13,376 lines became about 9,040); the 598 keys named
  mechanically from their first words (a trailing number or function
  word) are named by meaning; 22 strings no code used are gone; and
  `lint-strings` fails on a string no code uses. The section comments
  stay as the file's grouping.

### 7.1 Phase 4: faraday-core's layout

There is one rule a reader can predict: **an area's code is in its
folder**. The folder's `mod.rs` holds the area's state and logic, its
`screen.rs` draws it, and any further files hold its parts. `screens/`
holds the frame and the screens that belong to no area folder. The code
map in `lib.rs` has a table from every `Screen` variant to its file.

The moves from the old layout: moved to `docs/history/STRUCTURE-PASS-LOG.md`.

**Crate edges, measured in step 5.**
- **faraday-storage stays on faraday-core.** It is the layer between the
  shells and the app: it takes `&mut Faraday` and feeds stick events
  into it. The review's "a handful of types" missed that.
- **faraday-starter takes its strings from faraday-strings.** It still
  uses faraday-core for the PDF writer, version comparison, EDID, the
  mark and the blank backup sheet. Moving those out is a separate piece
  of work, done only if a reason comes up.

The steps: moved to `docs/history/STRUCTURE-PASS-LOG.md`.

### 7.2 Enforcement: the structure stays (F74)

Each rule this pass sets is held by `just`, so that a later change
cannot quietly undo it:
- **`costs-lint`, clippy's disallowed methods and the budget tests:**
  costly work goes through the token, and drawing does none.
- **`lint-strings`:** one strings file.
- **`lint-secrets`:** no printing and no `Debug` of secret types, and
  no secret types or secret-taking functions outside faraday-keys
  (`docs/KEYS.md`).
- **`lint-docs`:** status headers, and every path a doc names resolves.
- **`lint-tokens`:** the count of numbers written in place only falls.
- **`lint-layout`** (`tools/lint-layout.sh`, counts in
  `tools/lint-layout.counts`):
  - the code map: every module `lib.rs` declares appears in its `//!`
    code map, every `Screen` variant appears in the map's table, and the
    file named there exists and draws it;
  - file size: no Rust source file over 2,500 lines, except those in
    the counts file at their size, which only falls;
  - function size: clippy's `too_many_lines` is on, each long function
    carries an `#[expect]`, and their number is in the counts file; the
    same holds for `#[allow(clippy::too_many_arguments)]`;
  - `Faraday`'s top-level field count, which only falls;
  - the files allowed an inherent `impl Faraday`: the list only shrinks.
- **`cargo vet --locked`** in `just lint` (F71), **`cargo-deny`**, fmt,
  and clippy with warnings as errors.

**Every ratchet works the same way.** A count that rises fails the
lint, and a count that falls fails it too, until the file is lowered.
Raising a count is the owner's decision, recorded in DECISIONS, as
accepting more than a standard requires already is.

**CLAUDE.md names each rule and its lint** (its "Structure" section),
so a session sees the rules before it writes code.

**What lints cannot check is judgement.** That rests on CLAUDE.md's
process: a spec first for anything that takes decisions, design by
Opus, review from the tree. A periodic structure review, like the one
this pass came from, is on the board.

### 7.3 Phase 7's shape

- Each area's state is one struct in its folder (`VaultArea`,
  `BackupFlowArea`, and so on) with private fields, so Rust itself
  refuses a reach across areas. Reads from outside go through accessors
  (a read-only `Deref`, or a named method); writes are the area's own
  methods.
- What an area needs from elsewhere is passed in, so its signature
  states its reach. An area that is not given `&Keys` cannot reveal a
  secret.
- Flows that span areas are short `impl Faraday` functions that call
  each area in turn: explicit glue, in each area's own files, so the
  list of files with `impl Faraday` only shrinks. The stricter design,
  where areas only exchange messages, is not taken.
- The measures are `Faraday`'s top-level field count and the number of
  files with `impl Faraday`; both only fall, held by
  `tools/lint-layout.counts`. `Faraday` has 85 top-level fields (190
  before phase 7) and 61 files hold `impl Faraday` (72 before). The
  queues to the shell are one `Out`, the pointer's ten fields one
  `Pointer` and the idle lock's six one `Idle`; the 13 fields the shells
  set stay separate public fields.
- Two areas keep their types in an `area.rs` beside `mod.rs`
  (`faraday/faraday-core/src/vault/area.rs`,
  `faraday/faraday-core/src/backup_flow/area.rs`), because their
  `mod.rs` is at its line count.
- Several areas still hand out a transitional mutable accessor:
  `Camera::scan_mut`, `EntryArea::state_mut`, the `state_mut` of
  Create, Restore and Seeds, Wallets' `spend_mut`, `renaming_mut` and
  `card_offset_mut`, `VisitArea`'s `DerefMut` to `VisitState`, Glance's
  `work_mut`, Inbox's `potential_mut` and `skip_mut`, `files_menu_mut`,
  `vault_from_mut`, Family's `seeds_mut` and `scroll_y_mut`, the wallet
  backup checklist's `board_mut`, `check_mut` and `scroll_y_mut`, the
  vault's `item_offset_mut`, `create_scroll_y_mut` and `text_box`, and
  the backup flow's `board_mut`, `scroll_y_mut` and
  `board_list_offset_mut`, Message's `scroll_y_mut` and boot import's
  `state_mut`. These are to be narrowed into named methods
  (planned, not scheduled).

### 7.4 Phase 7's plan

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

#### 7.4.1 The rules each area commit follows

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

#### 7.4.2 The inventory

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

#### 7.4.3 Per area

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

#### 7.4.4 The order

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

#### 7.4.5 The ratchet

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

#### 7.4.6 Risks

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

#### 7.4.7 Out of scope

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

## 8. Order, checks and builds

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

## 9. Questions for the owner

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.
