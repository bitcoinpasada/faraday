# The structure pass: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/STRUCTURE-PASS.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## The header and the opening (from docs/STRUCTURE-PASS.md)

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

## §1 What this changes in the plan (from docs/STRUCTURE-PASS.md)

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

## §2 Phase 0: what is false and what is personal (from docs/STRUCTURE-PASS.md)

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

## §3 Phase 0b: the hard fork (F72): the order within the phase (from docs/STRUCTURE-PASS.md)

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

## §3.1 Names (from docs/STRUCTURE-PASS.md)

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

## §3.2 What the rename must not touch: what it was expected to change, and its check (from docs/STRUCTURE-PASS.md)

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

## §3.3 Provenance, frozen before the rename (from docs/STRUCTURE-PASS.md)

This was done on the commit just before the rename: the base hash and
the per-crate diffstat against it were written down, and the `upstream`
remote and the private for-upstream folder were then removed. From F72 on,
Faraday's own history is the record; the written-down provenance is no
longer kept.

## §3.4 Made Faraday's (from docs/STRUCTURE-PASS.md)

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

## §4 Phases 1 and 2: maps and the docs layout (from docs/STRUCTURE-PASS.md)

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

## §5 The key boundary (F70) (from docs/STRUCTURE-PASS.md)

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

## §7 Phases 3, 4, 6 and 7: phase 3 (from docs/STRUCTURE-PASS.md)

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

## §7.1 Phase 4: the moves (from docs/STRUCTURE-PASS.md)

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

## §7.1 Phase 4: the steps (from docs/STRUCTURE-PASS.md)

The steps, each its own commit with `just` green and the literal and
test-list checks unchanged:
1. the folders;
2. `lib.rs`'s parts;
3. `screens.rs`;
4. `act_inner` as a dispatch;
5. the crate edges;
6. the snapshot tours.

## §8 Order, checks and builds (from docs/STRUCTURE-PASS.md)

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

## §9 Questions for the owner (from docs/STRUCTURE-PASS.md)

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

## Phase 6 and 7 (2026-10-23)

Moved from `docs/STRUCTURE-PASS.md` when phase 7 was built (49 steps, commits 11692405 to 2f71ca21).

## §6 The measurements behind the dependency policy

The measurements that led to F71 (2026-10-21, 3bd25c2d):

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

### 7.4 Phase 7's plan

**Choices taken by default** (2026-10-23; each the one that changes
least and raises no count; the owner may change any of them):
1. Glue lives in each area's own main file, already listed, so the
   `impl Faraday` list only shrinks; no central `glue/` folder.
2. `commands`, `storage_out`, `dirty`, `toast` and `toast_at` are one
   `Out`, handed to areas as `&mut Out`: the existing queues to the
   shell, grouped, not message passing.
3. The 13 fields the shells set stay separate public fields: the shells'
   API does not change.
4. Screens in `screens/` that draw an area stay where they are and read
   it through accessors.
5. The spine groups `Pointer` and `Idle` (steps 47–48) are done last, if
   at all.

Measured on main at d4aa1175. Paths in this section are under
`faraday/faraday-core/src/` unless they start with `faraday/` or
`tools/`.

- `Faraday` has **190 top-level fields**, counted as §7.4.5 counts them
  (one more than a count by eye, which misses `texts_parsed`, whose type
  wraps to the next line).
- **72 files** hold `impl Faraday`, in 103 blocks with 1,199 functions;
  `lib.rs` alone holds 226 of them in 7,674 lines. One of the files,
  `keygen/mod.rs`, writes it `impl crate::Faraday`, which a search for
  `impl Faraday` misses (so 71 by that count).
- faraday-core's own tests (`faraday/faraday-core/tests/all/`) reach the
  fields that move into areas about 1,550 times, about 90 of them
  writes, two thirds of all of them the vault (506) and the wallet backup
  checklist's `backup` (227).
- About 115 `Faraday` methods are called from outside the crate's
  source (the shells, faraday-storage, faraday-samples, the tests).

#### 7.4.1 The rules each area commit follows

**The area struct.** Each area's state is one struct in its module: the
folder's `mod.rs`, or the file for a module that is one file
(`inbox.rs`). It is named `<Name>Area`; the spine's parts (§7.4.2) take
plain names (`Trail`, `Out`). Its fields are private, so Rust refuses
any reach from outside the module and its children. An area's
`screen.rs` and its other files are children, and read the fields
directly.

A private field on `Faraday` itself guards nothing inside the crate:
`lib.rs` is the crate root and every module is its child. The area
struct's own private fields are what make the boundary.

**Existing state types become fields of the area struct, unchanged
inside.** These are `vault::Vaults`, `family::FamilyState`,
`inbox::InboxFiles`, `VisitState`, `EntryState`, `LightningState` and
the rest. An `Option<XState>` "on show" (Lightning, BIP-85, Silent,
Explore, Vanity, the word list, New key, Create, Restore, Decode, the
backup screen, the upgrade) becomes the area struct's private field;
opening and closing the area are its methods.

**Reads from outside go through accessors.** An accessor returns `&T`,
or a value of a `Copy` type, and never `&mut`. A shared reference to a
type with public fields lets the reader read every field and change
none, so `app.vaults().open[0]` reads as `app.vaults.open[0]` did. The
exceptions are the fields that change through a shared reference (a
`Cell` or `RefCell`): `content_h`, `pin_h`, the kept values (§7.4.3,
`Kept`) and the drawing memo. Each is written where it is written
today, and no new one is added.

**Writes are only the area's own methods.**

**Moves and glue.** A function moves onto the area struct when it reads
and writes only that area's fields and what it can be given (the next
list). It stays on `impl Faraday` as glue when it:
- changes another area's state;
- calls another area's logic;
- dispatches (`act`, `act_inner`, `storage`, `event`); or
- is a public method other crates call (a forwarder, below).

Glue is short. It mints the token, calls each area in turn with what
each needs, and sets the screen. A long `<area>_act` is split: the arms
that touch only the area move, as `XArea::act`, and the glue keeps the
order of the calls. Behaviour depends on that order: a toast after a
save, a screen set after a load.

**What a moved function is given in place of `self`:**

| It used | It is given |
|---|---|
| `self.compute()` | `c: &Compute`, minted by the glue through `Faraday::compute` (the listed mint site, `docs/COMPUTE.md` §3). An area cannot mint. |
| `self.session` | `&Keys` (`&self.session.keys`) when it uses only keys; `&Session` when it reads wallets; `&mut Session` only when it loads or drops keys or wallets. |
| `self.toast(…)`, `commands`, `storage_out`, `dirty` | `out: &mut Out` (§7.4.4, step 1), and `now: u64` for a toast |
| `self.now_ms`, `medium`, `compact`, `online`, `tz_min` | the value (`u64`, `Medium`, `bool`, `i16`) |
| `self.screen`, `self.sheet`, to read | the value (both are `Copy`) |
| `self.screen`, `self.sheet`, to set | `&mut Screen` or `&mut Option<Sheet>`, or it returns where to go and the glue sets it |
| `may_load_keys()`, `makes_secrets()`, `selftest_passed()` | a `bool` the glue works out |
| another area's state, read | that area's accessor result (`&InboxFiles`, `&Kept`) |
| `put_outbox`, `save_boxes`, `offer_secret`, `vault_push`, `refresh_spend`, `act` | nothing: the area returns what it made and the glue calls these |

A moved function stays within clippy's seven arguments. One that needs
more is split, and the part that spans areas goes to the glue. No
`allow(clippy::too_many_arguments)` is added (`lint-layout` counts
them). No context struct bundling the spine is passed: it would hide
the reach the signature is meant to state.

**Forwarders.** Every `Faraday` method another crate or a test calls
keeps its name and signature as a forwarder of a few lines that hands
the area its pieces. That covers the shells, faraday-storage,
faraday-samples, faraday-core's examples and `tests/all`. A forwarder
lives with the area's glue. An area with no other glue puts its
forwarders in `lib.rs`'s last `impl Faraday` block, so that its folder
holds no `impl Faraday`.

**Drawing reaches an area as it reaches `Faraday` now** (`docs/COMPUTE.md`
§4). `View` derefs to `Faraday`, and gets no `DerefMut`.
- **Screens** read through the area's accessors. A screen in the area's
  folder reads the fields directly.
- **Memo fills and kept values.** `drawing/memo.rs`'s fills and
  `drawing/kept.rs`'s kept values call the area's `*_now` and kept
  methods with the view's token. A fill that spans areas
  (`backup_items`, `board_view`, `flows_in_progress`) calls the
  `Faraday` glue of the same name, as it does today.
- **`apply_after`** (`drawing.rs`) applies what the pass noted through
  each area's setter (`LearnArea::set_scroll`).
- Decisions keep using the `*_now` forms. Drawing computes nothing new.

**Tests.**
- **Reads.** A read of a moved field becomes an accessor call:
  `app.vaults.open` becomes `app.vaults().open`, and `app.backup`
  becomes `app.backup()`.
- **Writes.** A write becomes a function in `testkit.rs`: the 58
  `app.vaults.ms_per_unit = …` become `testkit::vault_speed(&mut app,
  ms)`, and the snapshot tours' writes take the same functions (they
  build with `testkit`).
- **Budget tests.** The budget tests in `work/budget/` are in the crate
  and change the same way.
- **No public fields for tests.** No field is made `pub` or `pub(crate)`
  for a test.
- **No other test changes.** No assertion changes, and the test count
  stays as it was.

**Each commit is one area and behaviour-identical:**
- no string, layout or kept byte changes;
- the only test edits are the access paths above;
- `just` is green;
- `tools/lint-layout.counts` is lowered: the field line, the `impl
  Faraday` lines, and the line counts of `lib.rs` and the other listed
  files that shrank.

#### 7.4.2 The inventory

Every field is assigned by where it is read and written (`self.<field>`
in `impl Faraday` and `impl View`, and `app.<field>` everywhere under
`faraday/`). An area takes the fields only it, its screens and its
tests use. A field many areas read but one area writes goes to the
writer, and the readers are given it or read its accessor.

**The areas** (45 structs, 132 fields):

| Area (module) | Struct | Fields |
|---|---|---|
| `drawing/kept.rs` | `Kept` | `public_qrs`, `texts_read`, `config_names`, `texts_parsed`, `message_checked`, `plans_mapped`, `seeds_addresses`, `fp_memo` (8) |
| `fresh.rs` | `Fresh` | `seed`, `seed_wait_from`, `seed_missing_drawn`, `sign_draws`, `pool` (5) |
| `stick/settings.rs` | `Settings` | `scale_pct`, `theme`, `reduce_motion`, `idle_lock_min`, `qr_frame_ms`, `idle_off_min`, `tz_min`, `guided` (8) |
| `trail.rs` | `Trail` | `trail`, `entered`, `backing`, `forward`, `landed`, `back_link`, `back_hit` (7) |
| `edit.rs` | `Typing` | `edit`, `editing`, `shift`, `fields`, `caret_at`, `caret_drawn` (6) |
| `motion.rs` | `Scroll` | `motion`, `motion_for`, `extent`, `region`, `bar_seen`, `sheet_scroll`, `content_h` (7) |
| `fade.rs` | `fade::Fade`, grown | `fade`, `last_drawn`, `frost` (3) |
| `inputs.rs` | `InputsArea` | `inputs`, `ignored_inputs`, `inputs_kept_out`, `input_decisions` (4) |
| `memory.rs` | `Memory` | `signed_amounts`, `seal_amounts`, `checked_wallets` (3) |
| `set_time.rs` | `SetTimeArea` | `set_time` (1) |
| `tools/` | `ToolsArea` | `tools`, `catalog_find` (2) |
| `lightning/`, `bip85/`, `silent/`, `explore/`, `vanity/`, `wordlist/`, `transfer/` | `<Name>Area` each | one field each, the area's state (7) |
| `learn.rs` | `LearnArea` | `learn`, `learn_ticks` (2) |
| `upgrade/` | `UpgradeArea` | `upgrade`, `upgrade_after_lock` (2) |
| `samples/` | `SamplesArea` | `sample_sheet` (1) |
| `modes/` | `ModesArea` | `next_mode` (1) |
| `secrets.rs` | `SecretsArea` | `secret_out`, `secret_ack` (2) |
| `keygen/` | `KeyGenArea` | `keygen`, `new_keys`, `keygen_camera_on`, `entropy_source` (4) |
| `seeds/` | `SeedsArea` | `seed_read` (1) |
| `compact/` | `CompactArea` | `osk`, `about_open` (2) |
| `boot_import/` | `BootImportArea` | `import`, `import_load_after`, `import_under_input` (3) |
| `scan_key.rs` | `Camera` | `scan`, `scan_parts`, `cameras`, `camera`, `camera_change`, `scanned` (6) |
| `qr_view.rs` | `QrArea` | `qr` (1) |
| `decode.rs` | `DecodeArea` | `decode` (1) |
| `state.rs` | `MessageArea` | `message`, `checking` (2) |
| `word_entry.rs` | `EntryArea` | `entry` (1) |
| `create.rs` | `CreateArea` | `create` (1) |
| `restore.rs` | `RestoreArea` | `restore` (1) |
| `family/` | `FamilyArea` | `family` (1) |
| `glance/` | `GlanceArea` | `glance`, `chart`, `chart_focus`, `chart_work` (4) |
| `wallet.rs` | `WalletsArea` | `wallet`, `loose_pick`, `renaming`, `card_offset`, `spend` (5) |
| `inbox.rs` | `InboxArea` | `inbox`, `inbox_skip`, `potential`, `typed_at` (4) |
| `stick/` | `StickArea` | `sticks`, `not_now`, `stick_held`, `stick_problems`, `stick_toggles`, `stick_settings`, `delete_ask`, `zeroing` (8) |
| `visit_taken.rs` | `VisitArea` | `visit`, `pull`, `after_visit`, `receipt`, `vault_from`, `files_menu`, `files_out_offset` (7) |
| `backups/` | `BackupsArea` | `backups_at`, `backup_records` (2) |
| `in_progress.rs` | `InProgress` | `flows_begun` (1) |
| `wallet_backup.rs` | `WalletBackupArea` | `backup` (1) |
| `vault/` | `VaultArea` | `vaults`, `vault_item_offset` (2) |
| `backup_flow/` | `BackupFlowArea` | `backup_flows`, `flow_on`, `owner_seq`, `board_list_offset`, `made_here` (5) |

The judgement calls, each from the field's readers and writers:
- **`entropy_source`.** It is written from `Event::EntropySource`
  (`keygen/mod.rs`, `entropy_source_named`) and read only by New key
  when it opens, so it goes to keygen.
- **`made_here` and `owner_seq`.** These go to the backup flow:
  `held.rs` and `backup_flow/wallet.rs` are their only users.
- **`board_list_offset`.** The board is the backup flow's (`board/`
  holds no state of its own), so its offset goes there too.
- **`backup_records`.** It goes to backups, and `memory.rs` keeps it
  across a lock through glue.
- **`stick_settings`.** It is what the stick holds, so it goes to the
  stick area, and the settings glue reads it.
- **`fp_memo`.** It is a cache by hash of the secret, read by `inbox.rs`
  only, and goes to `Kept` with the other caches. It is never written
  out, as today.

**The spine** stays on `Faraday`. Three groups of it become one field
each:
- **`Out`** (`lib.rs`): `commands`, `storage_out`, `dirty`, `toast`,
  `toast_at`. It is §7.4.4's first step.
- **`Pointer`** (`lib.rs`): `hits`, `pressed`, `last_tap`, `down_at`,
  `drag`, `swallow`, `bar_held`, `scroll_at`, `hover`, `hovered`.
- **`Idle`** (`lib.rs`): `last_input`, `idle_carry`, `idle_locking`,
  `lock_for_flow`, `idle_shown`, `idle_warn_min`.

The rest stay as they are:
- the screen and its navigation: `screen`, `sheet`, `sheet_offset`,
  `list_offset` (the stick shell reads it and a tour sets it);
- the session: `session`, which holds the keys (F70);
- the Outbox, which every area puts to: `outbox`;
- the frame: `canvas`, `f`, `w`, `h`, `compact`, `last_display`,
  `clock_drawn`, `pin_h`, `drawing`, `later`, `later_drawn`,
  `disclosed`, `guided_moving`;
- the clock: `now_ms`;
- the process: `restart`, `restart_computer`, `selftest`, `tainted`;
- what the shells set, which stays as it is so no shell changes:
  `online`, `seed_line`, `held`, `shell_sticks`, `clipboard`, `medium`,
  `secure_boot`, `maker`, `maker_family`, `maker_model`, `devices`,
  `samples`, `apk_signing`.

#### 7.4.3 Per area

The tables count, for each area, the functions that move onto its
struct and those that stay as glue. Each area also takes the functions
in `lib.rs` that touch only its fields: about 100 of `lib.rs`'s 226,
the largest groups being the wallet backup checklist's 16, the vault's 13,
Create's 11, the Inbox's 11 and the visit's 8. The counts include
those. Glue is named where it is short. Each test count is
`app.<field>` uses in `tests/all`, which become accessors.

**The spine's parts:**

| Area | Moves | Stays glue, and why | Tests |
|---|---|---|---|
| `Kept` | the six kept lookups (`public_qr`, `read_kept`, `config_name_kept`, `parsed_kept`, `message_check`, `plan_mapped`), `seeds_address` and `fingerprint_memo` | none. Callers in other areas are given `&Kept` | the cache tests in `work/budget/caches.rs` |
| `Fresh` | 5 | `fresh_arrived` (inputs), `seed_tick` (the canvas) | none |
| `Settings` | 4 | `settings_event`, `visit_settings_on`, `settings_on_stick_known` (stick and visit), `set_scale_kept` (the display). `kept`/`restore_kept` read it through `Settings::body` | 16 reads (`app.theme` and others); the snapshot tours read `app.theme` |
| `Trail` | 9 | `land`, `spot` (each screen's scroll: wallet, vault, backups, board, visit), `back`, `as_back_at`, `back_offered` (the pointer's hits), `shows`, `trail_name` | none |
| `Typing` | 3 | 11. Where typing goes (`typing_field`, `typing_text`, `typing_target`, `can_show`) reaches every area that has a field | none |
| `Scroll` | the motion arithmetic | `screen_slot`, `page_slot` (every area's scroll) | the scroll tests read `list_offset`, which stays |
| `fade::Fade` | 4 | `fade_begin`, `fades` (settings, Learn), `fade_step` (the canvas) | none |
| `InputsArea` | 6 | `input_code` (fresh), `input_new`, `input_sheet` (boot import) | 11 reads |
| `Memory` | 5 (the amounts, the checked wallets) | `kept`, `restore_kept`: what is kept across a lock is every area's | through `settings_file` and `vault_seal` |

**The areas whose glue becomes the dispatch arm** (no `impl Faraday`
left in the folder):

| Area | Moves | Glue left, in `lib.rs` | Given | Forwarders kept | Tests |
|---|---|---|---|---|---|
| word list | 2 | `wordlist_key` | sheet, `out` | none | 6 |
| Learn | 8 | `screen_pages` (vault, wallet) | sheet, screen, `out`, medium, online, `&Session` | `learn_pages` | 9 |
| Lightning | 5 | `lightning_act`'s save to a vault | `c`, `&Keys`, screen | none | 11 |
| Vanity | 6 | `vanity_use` (wallet) | `c`, `&Session`, `out`, now | none | 8 |
| Silent | 13 | `silent_act`'s save and Outbox, `silent_check` (Inbox) | `c`, `&Session`, `out` | `silent_address`, `silent_key_here` | 6 |
| BIP-85 | 6 | `bip85_out`, `bip85_to_vault` | `c`, `&Keys`, screen, makes secrets | none | 0 |
| Explore | 15 | `explore_x` (clipboard, list offset) | `c`, `&Session`, `out`, now | five `explore_*` | 24 |
| Tools | 6 (`catalog.rs` with it) | `catalog_go`, `tile_need` (vault, Inbox, samples, flows) | `&Session`, screen, `apk_signing` | `tile_need` | 7 |
| Transfer | 7 | `transfer_receive`, `transfer_saved`, `transfer_scanned` (camera), `transfer_open` | `c`, `out`, online, medium | none | 4, and faraday-storage's transfer test |
| Upgrade | 8 | `upgrade_open`, `upgrade_data_of` (sticks), `upgrade_act`'s way back | `out`, online | `upgrading`, `upgrade_data_of` | 1, and faraday-storage's upgrade test |
| Modes | 6 | `mode_act`, `save_boxes_for_lock` | `&mut Session`, `out` | `mode`, `modes`, `set_modes`, `mode_after_restart` | through `modes` |

**The areas that keep a glue block in one file** (their other files
lose theirs):

| Area | Moves | Glue file | Glue, and what it reaches | Tests |
|---|---|---|---|---|
| Set the time | 5 | `set_time.rs` | 7: the clock is kept in a vault and the zone is a setting | 1 |
| Small panel | 9 | `compact/mod.rs` | `osk_auto` and `osk_field` (every typing area), the files lines | 2 |
| QR view | 0 | `qr_view.rs` | `qr_act` (Outbox, backups, settings) | 12 |
| Camera | 7 | `scan_key.rs` | where a scan goes (`wallet_key_added`, `pasted`, `seed_read_open`) | 27 |
| Decode | 0 | `decode.rs` | `decode_act` (Inbox, the wallet) | 1 |
| Message | 0 | `state.rs` | `message_act` (Inbox, Outbox) | 3 |
| Entry | 4 | `word_entry.rs` (`forms.rs` loses its block) | `entry_act`, `form_recover`, `form_add_part` | 32 |
| New key | 7 | `keygen/mod.rs` | 13: a new key goes to Create, Entry or the session, the camera and fresh answers feed it, and its lock marks it made here (backup flow) | 104 |
| Secrets | 5 | `secrets.rs` | `offer_seed`, `secret_to_vault`, `secret_unprotected`, `vault_round` | 15 |
| Create | 16 | `create.rs` (`create_from.rs` loses its block) | `create_act`, `create_from_wallet`, `create_take_back` (flows, vault) | 109 |
| Restore | 4 | `restore.rs` | `restore_act` (entry, Inbox, wallet, flow) | 39 |
| Seeds | 9 | `seeds/mod.rs` (`seeds/read.rs` loses its block) | 19: a seed read goes to Family, Restore or the wallet | 4 |
| Wallets | 5 | `wallet.rs` | `wallet_act`, `refresh_spend`, `sign_here` | 74 |
| Glance | 10 | `glance/sheet.rs` (`chart_edit.rs` loses its block) | 17: the chart's changes save to a vault and the plan | 17 |
| Inbox | 26 | `inbox.rs` | 10: loading keys into the session | 90 |
| Boot import | 6 | `boot_import/mod.rs` | 11: the import unlocks a vault and loads | 20 |
| Stick | 13 | `stick/wipe.rs` (`stick/pills.rs` loses its block) | 19: the pills and wipes name the backup flow's slots | 16 |
| Visit | 14 | `visit_taken.rs` (`file_menu.rs` loses its block) | 12: `visit_act`, `visit_write`, `file_act` | 63 |
| Samples | 7 | `samples/put.rs` | 27: putting a sample in fills every area's fields | via faraday-samples |
| Family | 23 | `family/mod.rs` | 32: the Spend tab loads the wallet, reads the Inbox, scans | 40 |
| Backups | 10 | `backups/mod.rs` | 9: each wallet's backup line reads the vault and the backup | 0 |
| In progress | 9 | `in_progress.rs` | 18: the flows begun are every flow's | through the flow tests |
| Wallet backup checklist | 24 | `wallet_backup.rs` (`paper.rs` and `copy_check.rs` lose theirs) | 9, `backup_act` the largest | 227 |
| Vault | about 95 | `vault/mod.rs` (`vault/load.rs`, `vault/out.rs`, `vault/strength.rs`, `codes/mod.rs`, `apk.rs` and `gpg.rs` lose theirs) | about 55: loading into the session, the Inbox's vault files, the Outbox, a stick's settings | 506 (61 writes) |
| Backup flow | about 120 | `backup_flow/steps.rs` (`backup_flow/mod.rs`, `backup_flow/wallet.rs`, `board/mod.rs` and `held.rs` lose theirs) | about 115: it reaches the vault (39 functions), the wallet backup checklist's state (36), Create, Restore, the Inbox and the sticks | 47, and `backup_flow_wallet.rs` and `backup_board.rs` through methods |

The forwarders this keeps:
- the vault's twelve (`vault_files`, `vault_rows_now`, `clock_now` …);
- the backup flow's fifteen (`flow`, `board_plan`, `backup_waiting`,
  `receive_line` …);
- Family's eight;
- the boot import's three;
- the stick's three.

`lib.rs`'s spine methods stay as they are: `press`, `storage`,
`poll_storage`, `offers`, `frame`, `display`, `drawn_texts` and the
rest.

#### 7.4.4 The order

From the most self-contained to the most entangled. An area goes after
every area its moved functions are given, so that it is handed an
accessor rather than a field reached through `self`.

1. **`Out`** first. Every area after it is handed `&mut Out` in place of
   four fields and `toast`.
2. **The leaves next.** These are `Kept`, the tool screens, the
   settings, the clock, the inputs, fade and motion. Each is used
   through one dispatch arm or one tick, and their tests are few.
3. **The typing and navigation parts.** Typing and the trail read many
   areas, so their glue stays in place, but their own state is small.
4. **The areas that load keys.** These are camera, entry, secrets, New
   key, Create, Restore, Seeds, Wallets, Glance and Inbox. Each needs
   `&mut Session` and the accessors of the areas before it.
5. **The sticks and the visit.** These are boot import, stick, visit
   and samples.
6. **The flows last.** These are Family, backups, memory, in progress,
   the wallet backup checklist, the vault and the backup flow. The backup
   flow reaches every area before it.

| Step | Area | Fields after | `impl Faraday` files after |
|---|---|---|---|
| 0 | the ratchet (§7.4.5) | 190 | 72 |
| 1 | `Out` | 186 | 72 |
| 2 | `Kept` | 179 | 72 |
| 3–9 | word list, Learn, Lightning, Vanity, Silent, BIP-85, Explore | 178 | 64 |
| 10–13 | Tools, Transfer, Upgrade, Modes | 176 | 59 |
| 14–19 | `Fresh`, `Settings`, Set the time, inputs, fade, `Scroll` | 154 | 56 |
| 20–22 | `Typing`, `Trail`, small panel | 142 | 56 |
| 23–25 | QR view, Camera, Decode | 137 | 56 |
| 26–28 | Message, Entry, Secrets | 135 | 55 |
| 29 | New key | 132 | 55 |
| 30–33 | Create, Restore, Seeds, Wallets | 128 | 53 |
| 34–35 | Glance, Inbox | 122 | 52 |
| 36–39 | boot import, stick, visit, samples | 107 | 49 |
| 40–43 | Family, backups, memory, in progress | 104 | 49 |
| 44 | the wallet backup checklist | 104 | 47 |
| 45 | vault | 103 | 41 |
| 46 | backup flow | 99 | 37 |
| 47 | `Pointer` | 90 | 37 |
| 48 | `Idle` | 85 | 37 |

Every row is one commit with `just` green. A row covering several
areas is that many commits, and its counts are those after its last.

**The end state.** 85 top-level fields: 45 areas, 3 spine groups and 37
spine fields. 37 files with `impl Faraday`:
- `lib.rs`, `compute.rs` and `drawing.rs`;
- `demo.rs`, `devices.rs`, `secure_boot/mod.rs` and
  `secure_boot/state.rs`, which hold what the shells set and Demo's
  rules across areas;
- one glue file for each of 30 areas.

Glue that a later area's move makes unnecessary leaves the list when
it can.

#### 7.4.5 The ratchet

Step 0 extends `tools/lint-layout.sh` with a fourth check, and
`tools/lint-layout.counts` with two kinds of line:

```
# Faraday's top-level fields: only falls.
fields Faraday 190
# Files allowed `impl Faraday`: the list only shrinks.
impl-faraday faraday/faraday-core/src/lib.rs
impl-faraday faraday/faraday-core/src/vault/mod.rs
…
```

**What the lint checks:**
- **Fields.** It takes the text of `lib.rs` from the line `pub struct
  Faraday {` to the first line that is `}`. It counts the lines that
  match `^    (pub(\([a-z]+\))? )?[a-z_][a-z0-9_]*:`, so a field whose
  type wraps to the next line counts once, and doc comments and
  attributes do not count. The count is held by the same `ratchet` as
  the file sizes: above the line fails, and below it fails until the
  line is lowered.
- **Files.** It takes every tracked `.rs` file under `faraday/` with a
  line matching `^impl(<[^>]*>)? (crate::)?Faraday \{`. This matches
  inherent impls only, however the type is named: `impl App for Faraday` and `impl Default for Faraday` do
  not count. A file that has one and is not listed fails ("a new `impl
  Faraday`: move it into the area's struct or its glue file"). A listed
  file that has none fails until its line is removed. A listed path that
  is no tracked file fails.
- **Raising.** Adding a line, or raising the field count, is the owner's
  decision, recorded in DECISIONS (F74), like every other count.

`§7.2` item 3 is then built by this lint and by the area structs'
private fields. CLAUDE.md's "Structure" section gains the rule in the
same commit.

#### 7.4.6 Risks

- **Borrows.** A field and another field borrow apart, so
  `self.vault.unlock(&c, &mut self.session, &mut self.out)` compiles.
  A `&mut self` method on `Faraday` borrows all of it, so glue that
  holds an area's accessor result (`let v = self.vaults();`) cannot then
  call `self.toast(…)`. Glue uses the fields (`self.vault.vaults()`),
  not `Faraday`'s accessors, and copies out what it needs before the
  next call. A moved function that took `&mut self` and called a
  `&self` helper of another area is split, not given a clone.
- **The token.** `Faraday::compute` takes `&mut self`. The glue mints
  once, at the top, before it borrows any area: `let c =
  self.compute();`. That is what `act_inner` does today.
  `tools/costs-lint` holds the moved functions as it held them: a
  costly one still takes `c: &Compute` and says "Costs …".
- **The drawing memo** (`docs/SPEED.md` §3.3).
  - The memo and the kept values stay keyed as they are.
  - A kept value must move with its source, into the same area, so
    that "kept in step at every change" stays in one module. The
    vault's records stay behind `DerefMut` in `Vaults`, and an Item's
    wallet stays on the Item.
  - No fill may start writing through an accessor: drawing reads.
  - A function that moves from `Faraday` to an area keeps its `*_now`
    form, and the memo fill that called it calls the area's.
- **`act_inner`** (`lib.rs`, 473 lines) keeps its checks before any
  action, in their order:
  - the self-test;
  - the pull sheet for an action that loads a key;
  - the trail note;
  - Demo's refusals.
  Only its arms change, from `self.x_act(a)` to the area's call or its
  glue. Its `expect(clippy::too_many_lines)` stays counted. `later`
  (`docs/SPEED.md` §9) is the spine's, and an area never defers a
  press itself.
- **The budget tests** (`work/budget/`) count costly work per frame,
  press, key and pointer move. A move that mints a token in a new place
  or works something out twice fails them. They are run with every
  commit (`just` does), and none is loosened.
- **What is kept across a lock** (`memory.rs`, `kept` and
  `restore_kept`).
  - It writes name-and-bytes entries the next process reads (the
    settings, the signed amounts, the checked wallets, the backup
    records, each open vault's and each flow's state). Its glue reads
    every area through accessors, and its bytes must not change.
  - The lock tests (`vault_seal.rs`, `settings_file.rs`, the flow
    tests) hold them.
  - `storage_api.rs` carries the shells' storage events and is
    unchanged.
  - Nothing in faraday-core uses serde.
- **Secrets.** `Session` holds `faraday_keys::Keys`, so an area given
  only `&Keys` reaches what `docs/KEYS.md`'s eleven reveals allow, and
  one given neither cannot reach a key at all. `lint-secrets` rule 4
  and `lint-keys-tests` are unchanged. A `testkit.rs` function a move
  adds must not hand out a key.
- **Behaviour-identical.**
  - Each commit changes only access paths in tests, and the test count
    is compared before and after (`cargo nextest list`).
  - Where a split `*_act` reorders a toast, a screen change or a save,
    the tests may still pass, so the split keeps the calls in the order
    the old body made them, line for line.
  - One tour snapshot of the area's screen
    (`just faraday-snap 480x640 <tour>`) before and after is the check
    for drawing.
- **Size ratchets.** Moving functions out of `lib.rs` lowers its line,
  and the receiving file may pass 2,500 lines. `vault/mod.rs` and
  `backup_flow/steps.rs` are already listed and may not grow. An area
  whose module would pass the limit splits into files in its folder
  (`vault/area.rs`) in the same commit. No size line is raised.

#### 7.4.7 Out of scope

- No behaviour change, no new feature, no new screen, no renamed
  string.
- No message passing (§7.3).
- No move of a screen file: screens in `screens/` that draw an area
  (`screens/restore.rs`, `screens/spend.rs`, `screens/files.rs` and
  the like) read its accessors and stay where §7.1's table puts them.
- **No change to the API other crates use beyond what the move
  requires.** What they use today, all of which stays callable:
  - **Fields the shells set or read**, which stay on `Faraday`:
    - `faraday/shells/desktop` sets `online`, `seed_line`, `samples`,
      `shell_sticks` and `clipboard`, and reads `session` and `sheet`.
    - `faraday/shells/stick` sets `medium`, `devices`, `secure_boot`,
      `maker`, `maker_family` and `maker_model`, and reads `screen`,
      `devices` and `list_offset`.
    - `faraday/faraday-starter`'s tests set the four machine fields.
  - **Fields the snapshot tours and the samples' and storage tests
    reach**, which become accessors (reads) and `testkit.rs` functions
    (writes):
    - `faraday/shells/snapshot`: `vaults`, `backup_flows`, `inbox`,
      `outbox`, `message`, `backup`, `create`, `keygen`, `restore`,
      `bip85`, `vanity`, `spend`, `qr`, `scan`, `import`, `theme`;
    - `faraday/faraday-samples`' tests: `backup`, `create`, `entry`,
      `explore`, `keygen`, `lightning`, `potential`, `restore`,
      `scan`, `seed_read`, `vaults`, `spend`, `inbox`, `outbox`;
    - `faraday/faraday-storage`'s tests: `inbox`, `qr`, `scan`,
      `transfer`, `upgrade`, `visit`, `sticks`;
    - `faraday/faraday-core/examples/interop.rs`: `inbox`, `outbox`,
      `spend`.
  - **Methods.** The about 115 methods those crates and `tests/all`
    call (`press`, `storage`, `poll_storage`, `frame`, `offers`,
    `board_plan`, `vault_files` …) keep their names and signatures, as
    spine methods or forwarders. faraday-storage takes `&mut Faraday`
    as it does now.
  - **`tools/`.** Nothing reaches `Faraday`'s fields; only
    `tools/lint-layout.sh` and its counts change.
- **Not grouped in this phase.** The fields the shells set (`online`
  and the twelve beside it) stay as separate fields: grouping them is a
  shell API change, and the owner's decision.
