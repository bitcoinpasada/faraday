# The structure pass

**Kind:** Spec
**Status:** Partly built: phases 0 to 5 (the hard fork F72, the docs layout, phase 3's dead code, phase 4's layout, the key boundary F70) and F74's lints are built.
**Open:** phase 6 (`cargo vet --locked` in `just lint`, §6; faraday-intake's vector tests, §4; the strings' layout and names, §7; faraday-entropy split by source) and phase 7 (per-area structs, explicit glue, the `impl Faraday` ratchet, §7.2 and §7.3).
**Decisions:** F70 (the key boundary), F71 (dependency policy), F72
(the hard fork) and F74 (the structure enforced).

What is left of the pass that makes the repository easy to audit, and
the rules it set: the dependency policy (§6), the layout rule (§7.1),
the enforcement (§7.2) and phase 7's shape (§7.3).

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

Still to do (phase 6): faraday-intake gets vector tests.

What phases 1 and 2 did: moved to `docs/history/STRUCTURE-PASS-LOG.md`.

## 5. The key boundary (F70)

Built; the boundary is `docs/KEYS.md` (F70).

The plan as written: moved to `docs/history/STRUCTURE-PASS-LOG.md`.

## 6. Dependency policy (F71)

Measured 2026-10-21 at 3bd25c2d with cargo-vet 0.10.2, now installed on
the build machine.

- **`cargo vet --locked` fails today on 23 crates.** Most are crypto:
  - `curve25519-dalek`, `ed25519-dalek` and `ed25519`;
  - `rsa`, `crypto-bigint`, `crypto-primes` and `pkcs1`;
  - `pkcs8`, `spki`, `der` and `signature`;
  - `sha1` and `miniscript`;
  - the rest are their helpers and `autocfg`.
- **The six imported audit sets barely help.** `cargo vet prune` drops 3
  of the 223 exemptions. The audit backlog `cargo vet suggest` estimates
  is 5.1 million lines.
  - Most of it is the desktop window stack: winit, wayland, x11rb,
    windows-sys, objc2, and the Android crates winit locks.
  - So reading is feasible only for small crates (`sha1`, the PGP and
    APK helpers). Trust in publishers is the main route.
- **Trust entries expire.** `cargo vet trust` takes an end date at most
  one year ahead, so trust is renewed every year, and an expired entry
  fails `cargo vet --locked`. F71 states the renewal: the `vet-refresh`
  recipe, and a board item a month before the earliest end date.
- **rqrr needs a policy line.** Once the exemptions are pruned, cargo-vet
  refuses the vendored `third_party/rqrr` because it matches a published
  version: it needs `[policy.rqrr] audit-as-crates-io = false`. Faraday's
  patched copy is first-party code and is reviewed as such
  (`PATCHES.md`).
- **The publishers the imported organisations already trust.** cargo-vet
  names these for Faraday's tree, each trusted by Mozilla, ISRG and
  Bytecode Alliance:
  - dtolnay (syn, serde_json, itoa);
  - sunfishcode (rustix, linux-raw-sys);
  - kennykerr (windows-sys);
  - BurntSushi (memchr);
  - epage (winnow, toml);
  - alexcrichton (js-sys, web-sys);
  - rust-lang-owner (libc).

  F71 already names rust-bitcoin (apoelstra: bitcoin, secp256k1,
  miniscript), RustCrypto and dalek.
- **`--locked` everywhere:** F71 says every build passes `--locked`.
  The container builds (`faraday/build/linux`, `faraday/build/windows`)
  do, and so, since phase 6, do the stick and Pi binary builds, the
  desktop build and the Starter build in `faraday/faraday.just`, and
  `just`'s tests and clippy, so no build or check rewrites
  `Cargo.lock`.

## 7. Phases 3, 4, 6 and 7

Phase 3 (built: the unused functions deleted, the token backlog held by `tools/lint-tokens.counts`): moved to `docs/history/STRUCTURE-PASS-LOG.md`.

Still to do (phase 6):
- **Renaming the strings** (A11). It touches every screen, so it comes
  after the split (phase 4), as planned.

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
cannot quietly undo it. Already held:
- **`costs-lint`, clippy's disallowed methods and the budget tests:**
  costly work goes through the token, and drawing does none.
- **`lint-strings`:** one strings file.
- **`lint-secrets`:** no printing and no `Debug` of secret types.
- **`lint-docs`:** status headers, and every path a doc names resolves.
- **`lint-tokens`:** the count of numbers written in place only falls.
- **`cargo-deny`**, fmt, and clippy with warnings as errors.

Added as each phase lands:
1. **After phase 5:**
   - `lint-secrets` refuses secret types and secret-taking functions
     outside faraday-keys (docs/KEYS.md, step 4b). Built 2026-10-21.
   - tools/lint-layout.sh checks (built 2026-10-21; 209 long functions,
     72 `too_many_arguments` allowances and 11 files over the limit
     counted):
     - **The code map:** every module `lib.rs` declares appears in its
       `//!` code map. Every `Screen` variant appears in the map's table,
       and the file named there exists and draws it. A new top-level
       file in faraday-core is a new line in the map.
     - **File size:** no Rust source file over 2,500 lines, except those
       in tools/lint-layout.counts at their size, which only falls.
     - **Function size:** clippy's `too_many_lines` is on. Each long
       function carries an `#[expect]`, and their number is in the counts
       file. The same holds for `#[allow(clippy::too_many_arguments)]`
       (49 when this was written, 72 counted with tests and `core/`).
2. **Phase 6:** `cargo vet --locked` in `just lint` (F71).
3. **Phase 7:**
   - an `impl Faraday` ratchet: the files allowed one are listed, and
     the list only shrinks;
   - each area's state has private fields, so Rust itself refuses a
     reach across areas.

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

### 7.3 Phase 7's shape (owner, 2026-10-21)

- Each area's fields move into a struct in its folder (`VaultArea`,
  `BackupFlowArea`, and so on), and the area's logic moves from
  `impl Faraday` onto it.
- What an area needs from elsewhere is passed in, so its signature
  states its reach. An area that is not given `&Keys` cannot reveal a
  secret.
- Flows that span areas stay as short `impl Faraday` functions that
  call each area in turn: explicit glue.
- The stricter design, where areas only exchange messages (effects
  applied in one place), is not taken. It would rewrite every
  cross-area flow, the backup flow first, at real behaviour risk, for
  little more to audit than the glue gives.
- One area per commit, self-contained areas first.
- The measures are `Faraday`'s top-level field count and the number of
  files with `impl Faraday`; both only fall.

## 8. Order, checks and builds

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.

## 9. Questions for the owner

Moved to `docs/history/STRUCTURE-PASS-LOG.md`.
