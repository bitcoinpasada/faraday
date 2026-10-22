# The structure pass

**Kind:** Spec
**Status:** In progress: Waves A and B, phase 5 (the key boundary) and F74's layout lint built 2026-10-21; phases 6 and 7 to come, after other work (owner). §9's questions answered by the owner (2026-10-21).
**Verified against:** 3bd25c2d (2026-10-21)
**Decisions:** F70 (the key boundary), F71 (dependency policy), F72
(the hard fork) and F74 (the structure enforced), in DECISIONS.md; their
drafts are here.

This file is the plan for making the repository easy to audit: anyone
opening it knows where to look, and the code that touches secrets sits
behind one visible boundary. The plan has three parts, each of which
ends with `just` green and the four builds made: Wave A is the
documentation and the hard fork, Wave B is the dead code and the
monolith split, and Wave C is the secrets and the dependencies.

The phases keep the numbering of the review they come from (0, 0b,
1 to 7). §2 to §7 say what changes in that review's plan and why. §8
gives the order and the checks, and §9 lists the questions only the
owner can answer.

## 1. What this changes in the plan

1. **Two planned names are already taken.** `osk-ui` → `faraday-ui` and
   `osk-learn` → `faraday-learn` both collide with crates that already
   exist (`faraday/faraday-ui`, `faraday/faraday-learn`). §3.1.
2. **A blanket `osk-` → `faraday-` rename would change bytes on disk and
   on the wire.** Domain-separation tags, magics and file extensions
   contain `osk` or `OSK`. §3.2 lists them and says how the
   rename avoids them.
3. **Provenance has to be frozen before the rename, not after.** Once
   every crate and every `use` line is renamed, `git diff d26b84e HEAD
   -- core/` no longer shows Faraday's changes to the forked code. The
   per-crate diffstat and the auditor's diff are produced on the last
   commit before the rename. §3.3.
4. **AUDIT.md is written once, at the end.** The plan rewrote it in
   phase 0, and the rename, the osk-ui deletion and the key boundary
   would then change it three more times. Phase 0 now only corrects what
   is false today (it says faraday-core performs no cryptography). The
   full rewrite comes with F70, which is what an auditor reads. §2.
5. **osk-ui's dead widget tree is deleted before the rename.** That way
   about 17k dead lines are not renamed. The deletion is a soft-fork
   change, so it goes into the provenance record. §3.
6. **Test consolidation moves into Wave A.** It is the review's A9: one
   test binary instead of about 110. It was dropped from the plan, but
   every later step runs `just`, and each run links faraday-core about
   109 times. Doing it early makes every later step cheaper. §4.
7. **F71's premise "every build uses `--locked`" is false today.** The
   stick and Pi image builds (`faraday/faraday.just:283` and `:370`), the
   desktop build (`:143`) and the Starter build (`:217`) all run
   `cargo build` without it. Only the container builds pass it. §6.
8. **The key boundary needs a definition of "secret".** The scope says
   every secret. The tree also holds secrets that are not keys: typed
   input, camera frames, scanned SeedQR text, the Starter's BitLocker
   recovery password. The pass also has to say what happens to the
   crates that take a secret and keep nothing: faraday-pgp, faraday-sb,
   faraday-apk and faraday-vault. §5.
9. **Builds come at the end of each wave, not once at the end.** The
   campaign spans days. One build at the end leaves the menu apps a week
   old, and a broken image would surface far from its cause. §8.

## 2. Phase 0: what is false and what is personal

- Personal information and private references come out of the tracked
  docs with a normal commit, the lines the review listed. The tracked
  docs then carry no private-folder file names, tracker numbers or private
  repository names.
- `docs/AUDIT.md`: only its false statements are corrected. One base
  hash replaces the two it gives. `osk-keep` and the deleted app crate
  come out. faraday-core's key use is stated, with the files that do it.
  faraday-apk gets a row. The full rewrite comes in Wave C (§5.6).
- Broken doc paths are left for phase 2, where the moves and
  `lint-docs.sh` fix them in one commit. Fixing them now and then moving
  every file would do the work twice.

## 3. Phase 0b: the hard fork (F72)

Order within the phase:

1. Delete osk-ui's widget tree: the modules nothing in `faraday/` uses.
   The compiler confirmed this at 3bd25c2d. With `audit`, `components`,
   `descriptor`, `organisms`, `screens`, `state`, `gallery`,
   `widgets::chunked`, `widgets::render` and `Widget` made crate-private,
   the whole workspace still builds, tests included, and osk-ui reports
   378 dead items. Those modules are 13,430 lines. Most of
   `widgets/mod.rs`'s 4,273 lines go with `Widget`. What stays is about
   8k lines: canvas, color, geom, layout, text, tokens, fonts, and the
   icon and keyboard widgets. Faraday uses `Color`, `Rect`, `Canvas`,
   `SizeClass`, `tokens`, `widgets` (Icon and the keyboard), `geom`,
   `text`, `layout` and `fonts`. Move
   `LayoutCtx` and `Icon` out of `widgets`. This is a change to `core/`.
   Retargeting `lint-tokens.sh` stays in
   phase 3, because it brings a backlog of literals.
2. Freeze the provenance (§3.3).
3. Rename (§3.1, with §3.2's guard), move the camera shells and delete
   the old camera folder.
4. Make the descriptions, the `repository` keys, the `lib.rs`
   overviews, the Learn pages and the board Faraday's (§3.4).

### 3.1 Names

| Now | After | Note |
|---|---|---|
| osk-backup | faraday-backup | `.oskb` and the KDBX writer |
| osk-bip | faraday-bip | |
| osk-codec | faraday-codec | |
| osk-crypto | faraday-crypto | the `Compute` token lives here |
| osk-entropy | faraday-entropy | |
| osk-psbt | faraday-psbt | |
| osk-selftest | faraday-selftest | |
| osk-shell-api | faraday-shell-api | |
| osk-ui | **faraday-draw** (proposed) | collides with faraday-ui; §9 Q1 |
| osk-learn | **merged into faraday-learn** (proposed) | collides; §9 Q2 |
| osk-fontbake (`tools/fontbake`) | faraday-fontbake | |
| the old v4l2 crate | faraday-v4l2, `faraday/shells/v4l2` | next to faraday-mediafoundation |
| the old avfoundation crate | faraday-avfoundation, `faraday/shells/avfoundation` | |

`core/` keeps the libraries that hold no flow and no wording, whatever
their name. CLAUDE.md's paths follow in the same commit:
core/osk-ui/src/tokens.rs, core/osk-learn/src/en.rs and
core/osk-selftest.

Outside the repository, anything that depends on these crates by path
breaks at the rename and is changed the same day.

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

These are expected to change, and nothing else is:
- source paths that tools read: core/osk-crypto/src/work.rs in
  costs-lint, and core/osk-ui/assets and core/osk-ui/src/fonts/generated.rs
  in fontbake;
- the internal thread names (`osk-pi-watch-{}`, `osk-pi-fb-{}-{tag}`)
  and layout ids (`osk-home-receiver`, `osk-passphrase-keyboard`,
  `osk-passphrase-button`, `osk-tools-keyboard`). None of these is
  stored or sent anywhere;
- the KDBX `Generator`, `DatabaseName` and `Name` (§9 Q3);
- Learn page text (§3.4).

The check for the rename commit: the sorted list of string literals in
every `src/` is the same before and after, except the lines above.
A script kept with the pass writes that list. The baseline
at 3bd25c2d holds 56,000 literals, 88 of which name osk or the old project.
The step's report includes the diff. The vectors and the vault and
backup tests catch the rest.

### 3.3 Provenance, frozen before the rename

This was done on the commit just before the rename: the base hash and
the per-crate diffstat against it were written down, and the `upstream`
remote and the private for-upstream folder were then removed. From F72 on,
Faraday's own history is the record; the written-down provenance is no
longer kept.

### 3.4 Made Faraday's

- Each crate's `description` and the first line of each `lib.rs` say
  "Faraday's …" and name the fork point once.
- The root `repository` key is Faraday's.
- The mentions of the deleted app's core crate are fixed.
- The MIT notice stays in `LICENSE`.
- **Learn pages:**
  - Some pages describe the earlier app. The secure-element page,
    for example, covers keeping keys on an Android phone, which Faraday
    has no counterpart for.
  - A page Faraday never shows is dropped.
  - A page Faraday shows says Faraday where it means this app, and
    "an .oskb backup" only for the file format.
- **KDBX:** files written by the KDBX writer name Faraday as their
  generator and database name. §9 Q3.
- **CLAUDE.md:** F65's per-change rule ends. Its two lasting rules move
  into CLAUDE.md for the whole repository: crypto changes are tested
  against vectors or another implementation, and a check is never
  loosened.

## 4. Phases 1 and 2: maps and the docs layout

The plan's phases 1 and 2 stand, with these changes:

- **Test consolidation** (A9) comes first in this part. Each crate gets
  one `tests/main.rs` with a module per current test file. The copied
  helpers move into `tests/common`, so `type_text`, `shown`, `device`,
  `settle` and `add_key` are each defined once. faraday-intake gets
  vector tests. No test is deleted. Test names keep the file as a module
  prefix, so the count before and after is the check.
  - Done 2026-10-21 for faraday-core (110 binaries to 1), faraday-bip
    (27), faraday-psbt (14), faraday-starter (12), the Starter shell (10)
    and faraday-storage (8): `tests/all/main.rs`, one module per former
    file. All 2,394 tests are kept, under `<file>::` names.
    - Rebuilding the test binaries after a faraday-core change went from
      59 s to 10 s.
    - The identical copies of `type_text`, `shown`, `device` and
      `add_key` moved into `common`; copies that differ stay where they
      are.
    - faraday-intake's vector tests are still to do (phase 6).
- **`lint-docs.sh`:**
  - It skips `docs/history/` for path resolution.
  - It checks the status header everywhere.
  - Its first run lands in the same commit as the moves.
- **The status header:** this file's header is the form. Kind is
  Reference, Spec or History.

Phases 1 and 2 were done 2026-10-21:
- **Navigation:**
  - `docs/README.md`, the index from question to document;
  - the DECISIONS index and its status lines;
  - READMEs for `tools/`, the five crypto crates, `design/` and `faraday/build/`;
  - faraday-core's code map.
- **Layout:**
  - `docs/SECURITY-MODEL.md`;
  - the finished docs and the build logs moved to `docs/history/`;
  - `BACKUP-FORMAT.md` and rqrr's `PATCHES.md`;
  - the justfile's groups and fuzz recipe;
  - the stale worktrees pruned.
- **Lint:** a status header on every doc, and `tools/lint-docs.sh` in `just lint`.

F73 then removed the upstream planning documents and the provenance
file (§3.3) and every mention of the upstream project outside
`LICENSE`.

## 5. The key boundary (F70)

### 5.1 What counts as a secret

**Key material** is a seed, words, a BIP-39 passphrase, a private key or
anything derived from one, a TOTP secret, a GPG, APK or Secure Boot
private key, a FROST or MuSig secret nonce, a silent payment scan key,
and the open vault's KEY and PASSPHRASE records. faraday-keys owns all
of it.

**Transient secret input** is a typed word or passphrase, a camera
frame, scanned or pasted text, or a vault passphrase while it is typed.
It stays where it arrives, under lint-secrets' rules (`Zeroizing`, no
heap text, no `Debug`), and is handed to faraday-keys at the earliest
point:
- typed words at Done;
- a scanned SeedQR when it is classified;
- a vault passphrase at Open.

The Starter's BitLocker recovery password is not key material and stays
in the Starter (§9 Q4).

### 5.2 Which crates are involved

- **faraday-keys** holds the key material and does every operation on
  it. That includes PSBT signing (calling faraday-psbt), message
  signing, BIP-85, Lightning, GPG, APK, Secure Boot and TOTP. Its costly
  functions take `c: &Compute`. The token lives in faraday-crypto
  (today `osk-crypto/src/work.rs`), so no crate cycle results.
- **Primitive crates** take a secret by reference, return a result and
  keep nothing: faraday-crypto, -bip, -psbt, -entropy, -backup and
  -vault, plus faraday-pgp, faraday-sb and faraday-apk.
  - They stay separate crates.
  - Their secret-taking functions are called only from faraday-keys,
    which lint-secrets enforces.
  - Their public parts are open to everyone: parsing an APK, verifying a
    signature, reading a vault header.
  - §9 Q5 asks whether to fold them in instead.
- **faraday-core** holds handles (`KeyId`) and the kept values:
  fingerprints, xpubs, words id and shares. Drawing reads those and never
  a secret.

### 5.3 The way out

Words and passphrases leave faraday-keys only through
`reveal(id, Reveal::…)`, where the reason is one of:
- show the seed;
- copy it by hand;
- SeedQR;
- an unprotected secret to the Outbox.

The result is a guard that borrows the store and zeroes on drop. Adding
a reason means editing that enum, which shows in review.

### 5.4 The vault

In step 3, faraday-keys takes over the open vault's decrypted records
and the slot key that saves them again. faraday-vault keeps the file
cipher. Unlock splits what it opens: key records go to faraday-keys, the
rest to faraday-core. The vault format does not change, and the vault
vectors prove it.

### 5.5 The lint

Outside faraday-keys and the primitive crates, `lint-secrets.sh`
refuses:
- `MasterKey`, `SecretKey`, `Xpriv`, `Keypair`, `to_seed` and
  `Mnemonic`;
- a primitive's secret-taking functions.

`tests/` directories are exempt, because vector tests build keys.
faraday-intake's word check becomes a faraday-keys call.

### 5.6 What it changes

- AUDIT.md is rewritten around the boundary: the crates that compute,
  the one that holds, the reveal list, and how the lint enforces it.
- About 40 faraday-core source files name a secret type today. That
  count is the step's measure.
- Steps 2 and 3 touch unlock, so they are timed in a late-flow session
  before commit (SPEED §7).

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
- **`--locked` everywhere:** the image, desktop and Starter builds get
  `--locked` (§1 item 7) before F71 is written, so F71 states something
  true.

## 7. Phases 3, 4, 6 and 7

They stand as planned. Phase 3 no longer includes the osk-ui deletion
(moved to 0b) and no longer includes `publish = false` and the
descriptions (done in 0b).

Phase 3's unused functions, confirmed by the compiler at 3bd25c2d. Each
was made `pub(crate)`, the workspace still built with its tests, and the
compiler reported it dead:
- **faraday-core (16):**
  - `create::new_words`, which makes words from entropy;
  - `held::backup_stickless`;
  - `keygen::source_index`;
  - `boot_import::import_stick_present`;
  - `wallet::{words_are, public_seed}`;
  - `vaults::put_passphrase`;
  - `backup_flow::subject_label`;
  - `backup_flow_wallet::{wallet_backup_waits, wallet_public_on}`;
  - `plan::{set_row, remove_last_place, set_split, toggle_form,
    toggle_share, drop_old_file}`.
- **core (10, plus one constant):**
  - osk-bip: `diceware::max_chars`, `musig::{plain_key,
    derives_from_aggregate, aggregate_xpub_at}` and
    `silent::is_eligible`;
  - osk-codec: `qr::alnum_capacity`;
  - osk-crypto: `sealed::secp_blind`, and with it `SECP_BLIND_LABEL`
    (osk-bip's `keys.rs` keeps its own copy of the `osk-secp-blind`
    tag);
  - osk-entropy: `any_caution`;
  - osk-psbt: `classify::{is_ecdsa, is_sorted}` and
    `inspect::signature_count`.
- **Others (4):** faraday-qr `send::{animated_frames, one_code}`, and the
  Starter shell's `udisks::{is_filesystem, has}`.

Each is deleted, or kept with the reason written at the function.
`musig`'s three may be API kept on purpose; they are checked against
their tests before they go.

Two items need more than the plan said:
- **Retargeting lint-tokens at Faraday's screens.** The backlog was
  measured at 3bd25c2d. The lint's own rule (no decimal literal other
  than 0.0, 0.5, 1.0 or 2.0 outside comments, strings and tests) finds
  8,753 in faraday-core and faraday-ui, against 410 uses of a token:
  - screens.rs: 4,215;
  - vault_screens.rs: 1,153;
  - family_screen.rs: 490;
  - keygen_screen.rs: 434;
  - the rest is spread over about 60 files.

  The most common values are 12.0 (901), 13.0 (710), 14.0 (627) and
  8.0 (556). Most are text sizes and gaps. CLAUDE.md's rule "every
  dimension is a name in tokens.rs" is not true of Faraday's screens.
  Naming them all is a batch the size of the screens.rs split, and
  naming by value would merge numbers that only happen to be equal.
  §9 Q8 asks how to handle it.
- **Renaming the strings** (A11). It touches every screen, so it comes
  after the split (phase 4), as planned.

### 7.1 Phase 4: faraday-core's layout

There is one rule a reader can predict: **an area's code is in its
folder**. The folder's `mod.rs` holds the area's state and logic, its
`screen.rs` draws it, and any further files hold its parts. `screens/`
holds the frame and the screens that belong to no area folder. The code
map in `lib.rs` has a table from every `Screen` variant to its file.

| Now | After |
|---|---|
| `backup_flow`, `_screen`, `_steps`, `_wallet`, `_wallet_screen` | `backup_flow/{mod, screen, steps, wallet, wallet_screen}` |
| `vaults`, `vault_screens`, `vault_load`, `vault_load_screen`, `vault_out` | `vault/{mod, screen, load, load_screen, out}` |
| `seeds`, `seeds_screen`, `seed_read` | `seeds/{mod, screen, read}` |
| `silent`, `silent_screen`, `silent_check` | `silent/{mod, screen, check}` |
| `family`, `family_screen`, `family_text` | `family/{mod, screen, text}` |
| `glance`, `glance_sheet` | `glance/{mod, sheet}` |
| `plan`, `plan_cards` | `plan/{mod, cards}` |
| `secureboot`, `secure_boot_state` | `secure_boot/{mod, state}` |
| `stick_pills`, `stick_settings`, `stick_wipe` | `stick/{pills, settings, wipe}` |
| `compact`, `compact_screens` | `compact/{mod, screen}` |
| X and `X_screen`, for X in backups, bip85, board, boot_import, codes, explore, keygen, lightning, tools, transfer, upgrade, vanity, wordlist | `X/{mod, screen}` |
| `backup` (a wallet's backup, in two parts) | `wallet_backup` |
| `lib.rs`'s storage protocol, version, `Action`, QR view and receipt | `storage_api`, `version`, `action`, `qr_view`, `receipt` |
| `lib.rs`'s `*State` structs | their area's `mod.rs`, or `state.rs` where there is no area |
| `screens.rs` | `screens/mod.rs`: the `Screen` enum's dispatch, the sidebar, sheets and shared parts. Each inline screen goes to its area's `screen.rs`, or to `screens/<name>.rs` |

**Crate edges, measured in step 5.**
- **faraday-storage stays on faraday-core.** It is the layer between the
  shells and the app: it takes `&mut Faraday` and feeds stick events
  into it. The review's "a handful of types" missed that.
- **faraday-starter takes its strings from faraday-strings.** It still
  uses faraday-core for the PDF writer, version comparison, EDID, the
  mark and the blank backup sheet. Moving those out is a separate piece
  of work, done only if a reason comes up.

The steps, each its own commit with `just` green and the literal and
test-list checks unchanged:
1. the folders;
2. `lib.rs`'s parts;
3. `screens.rs`;
4. `act_inner` as a dispatch;
5. the crate edges;
6. the snapshot tours.

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

| Wave | Steps | Quiet tree needed | Builds after |
|---|---|---|---|
| A | 0 → 0b (osk-ui deletion, provenance, rename, Faraday's) → test consolidation → 1 → 2 | for 0b and the tests | desktop, stick, Pi and Starter: the rename reaches every binary |
| B | 3 → 4 | for 4 | all four |
| C | `--locked` → F70 steps 1–3 → F71 → 6 → 7 → AUDIT.md rewrite | for F70 and 7 | all four, plus the late-flow timing |

- Each step is one commit with `just` green.
- A step reported as done is checked from the tree, not from the
  report.
- Every binary to be published is checked by `strings` for local paths,
  as for a release.

**Preliminary measurements, done 2026-10-21 at 3bd25c2d** in a scratch
worktree that was then thrown away. Their results are in §3, §3.2, §6
and §7:
- make osk-ui's widget modules and faraday-core's 17 unused `pub`
  functions `pub(crate)`, and keep the list of what the compiler then
  calls dead;
- count the literal dimensions in faraday-core's and faraday-ui's
  screens (lint-tokens' backlog);
- `cargo vet` as it stands, `prune` and `suggest`, if cargo-vet is
  installed there;
- the string-literal baseline for §3.2's check.

## 9. Questions for the owner

Answered 2026-10-21: the owner took the recommendation on all seven.

1. **osk-ui's new name.**
   - (a) `faraday-draw` in `core/`, the renderer: canvas, text, fonts,
     tokens and the keyboard. Recommended.
   - (b) Merge what is left into `faraday-ui`, which makes one drawing
     crate. That costs osk-ui's `no_std` boundary.
2. **The two Learn crates.**
   - (a) Merge osk-learn into faraday-learn: one Learn crate, with the
     pages Faraday never shows dropped. Recommended.
   - (b) Rename osk-learn to `faraday-learn-pages` and keep both.
3. **Visible bytes that name the old project.** These are the KDBX file's
   `Generator` and `DatabaseName`, plus the stick's watch thread name.
   Change them to Faraday? KeePass shows the database name and ignores
   the generator. Recommended: yes. The tags and magics in §3.2 never
   change.
4. **Transient secrets** (§5.1): handed to faraday-keys at the earliest
   point, and the Starter's BitLocker password out of scope? Recommended:
   yes.
5. **Primitive crates** (§5.2): faraday-pgp, faraday-sb, faraday-apk and
   faraday-vault stay separate and lint-confined (recommended), or fold
   into faraday-keys?
6. **Builds at the end of each wave** instead of once at the end?
   Recommended: yes.
7. **Test consolidation (A9) in Wave A?** It was left out of the plan.
   Recommended: yes.

Raised by the measurements; answered 2026-10-21, the recommendation on both:

8. **The token backlog (§7).** Three options:
   - (a) A ratchet: the lint counts the literals in each crate and fails
     when the count rises above the committed number, which only goes
     down. A new screen file starts at zero. Phase 4 and later screen
     work name the literals in the code they touch. CLAUDE.md says the
     rule holds for new code and the count shrinks. Recommended.
   - (b) Name all 8,753 in one batch after phase 4.
   - (c) Limit the rule to osk-ui's remainder and say so.
9. **Whom to trust (§6).** Three options:
   - (a) The three F71 names, plus the publishers the imported
     organisations already trust. Everything else stays an exemption,
     which `docs/deps/` calls "not reviewed". Recommended.
   - (b) Only the three F71 names.
   - (c) Trust broadly, to empty the exemption list. That would claim
     more than anyone checked.
