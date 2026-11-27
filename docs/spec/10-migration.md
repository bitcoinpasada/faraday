# The migration method: how each of main's modules is copied, ported, rewritten or dropped

**Kind:** Spec
**Open:** Not built.

This section says how `main`'s code at `d89dde5de` becomes `next`'s. It gives the four fates a
module of `main` can take (copy, port, rewrite, drop) and what decides which; for each fate, what
the change does, what it brings with it (tests, decisions, documents, fixtures) and how it is
checked against `main`; the mapping that records every module's fate and when it is done; how one
inventory feature file becomes a feature's `Msg`, `State`, models and tests; the parity list; what
is never ported; the order the work must follow, as dependencies; and the cut-over. It answers the
questions sections 07, 08 and 09 left for it. It is written for whoever copies, ports or rewrites
a module of `main`, and for whoever reviews that change. Rules are numbered MG1 onward.

## 1. The frozen commit and its tag

`main` is frozen at `d89dde5de`. `next` is a branch with no common history (its first commit has
no parent), so `d89dde5de` is not an ancestor of `next`, and after the cut-over (§10) no branch
holds it.

**Decision: `d89dde5de` is kept by an annotated, signed tag, `history/main-2026-11`.** It follows
the tag `main` already keeps for its deleted documents (`history/docs-2026-11`), and it is made
before any check on `next` resolves a path at `d89dde5de`. Reason: a tag is the one git ref that
keeps a commit reachable on every clone and on GitHub once no branch holds it. Turned down: a
branch `main-frozen`, which reads as a branch to work on and can be moved by a push.

- **Rule MG1.** Every check that reads `main`'s tree (`lint-docs` rule 2, 08 §4.6; the mapping
  check, §4; `docs/crypto/check.py`, §8) reads it at the tag, and fails, saying so, when the tag
  is missing or does not peel to `d89dde5de`. The documents keep citing the hash; the tag is how
  a clone gets it. GitHub Actions' checkout fetches tags (`fetch-depth: 0`). Held by those checks.

## 2. The four fates

Every file of `main` at `d89dde5de` takes one fate. The default for a crate is its row in section
01 §9; a file is given another fate when its job differs from its crate's.

| Fate | What the change does | Decided when |
|---|---|---|
| **Copy** | brings the file to its path on `next` unchanged but for the named changes: an edge cut (01 §2.3), I/O moved out (01 §2.1), a moved directory, a path rewritten, a header added to a test it changes (D3) | the file's layer is the same on `next` and its code fits that layer as it is: the domain crates (09 §2.1), the kit's copied crates (`faraday-draw`, `faraday-ui`, rearranged by level, 09 §3.1), `faraday-shell-api` (grown into the port, 02), the process adapters and capture adapters, the I/O files of the shells that become adapters (the stick shell's `evdev.rs`, `hid.rs`, `touch.rs`, `files.rs`), the tools of 09 §8 but the test kit, the fuzz targets, `third_party/`, `supply-chain/`, the image build, the kept fixtures (§5.4), and the documents 08 §4.4 carries "as they are" |
| **Port** | carries the module's rules, refusals, state and screens into the new layers (a feature, a part, a composite, an organism or template, an owner), filling the skeleton's stubs; `main`'s code is read for its rules, and its code is not copied | the module holds application logic: `faraday-core` (but §2.3), `faraday-starter`, `faraday-host`, and the drawing of `faraday-core`'s screens and components |
| **Rewrite** | writes the module's job again to a contract the specification gives; `main`'s code is read only to confirm a fact | the module's shape is what the plan overturns, so carrying its code would carry that shape: the `Faraday` struct and its glue (`lib.rs`'s dispatch, every `impl Faraday` that only routes), the event loop (to the runtime, 05), continuations kept in state (to messages, 03), measures and hit targets written while drawing (to the frame, 04), the shells' main loops (to roots, 06), `faraday-storage`'s and `faraday-window`'s glue to `faraday-core` (to the port, 02), the scanner's (09 §6), the tour root (07 §5), the test kit's data (§6), and the documents 08 §4.4 lists as rewritten |
| **Drop** | brings nothing; the mapping row names why and, where the module held a rule, what holds that rule on `next` | the module has no job on `next`: the tests of ported crates (TS4), the lints of `main`'s structure (08 §8.2), `main`'s history documents but its decisions (08 §4.5), the examples and helpers that drive `Faraday` (§6, §7) |

- **Rule MG2.** The fate of a file is chosen by the table above and recorded in its mapping row
  (§4) before the change that carries it out. A file whose fate is unclear is not ported by
  guess: the specification section that owns its layer is changed first. Held by the mapping
  check (every file has one row) and by review.

**Decision: port and rewrite are two fates, not one.** A port's review reads `main`'s module
beside the new code and confirms each of its rules has a place (§5.2); a rewrite's review reads
the specification's contract, since `main`'s code for it is the shape being replaced. Reason: the
two reviews read different sources, and naming the fate tells the reviewer which. Turned down: a
single "port" fate, which would ask a review of the runtime to find each line of `Faraday::update`
in it.

### 2.1 Copy

- The change brings every file of the row at once, with the copied crate's own tests (on the
  copied-tests list, 09 §8) and its fixtures, and changes only what 01 §2.3, 01 §2.1 and 09 §2.1's
  last column name. A copied test is not edited beyond a path; a test that cannot build after the
  cuts is dropped and named in the row (§7 for `faraday-samples`).
- It brings the documents that describe the crate (08 §4.4, DOC4, DOC5), with their paths
  rewritten to `next`'s tree in the same change.
- **Rule MG3.** A copied file's difference from `main` is only its row's named changes.
  `python3 -I -B tools/migration.py diff <main path>` prints the difference between the file on
  `next` and at the tag, with each moved directory prefix (`faraday/faraday-keys/` to
  `core/faraday-keys/`) folded, so a file only moved shows none. Held by review of that output in
  the copying change.

### 2.2 Port

- The change fills the stubs of the crate 09 §10 names for the inventory file, from the
  inventory file (§5) and `main`'s modules named in the mapping rows it completes, read at the
  tag.
- `main`'s tests of the module are not ported and not read (TS4). Its fixtures carry (§5.4).
- The decisions that bind the feature are cited from the new code as they were from the old; the
  records are not edited (08 §5.2). A port whose design differs from a record follows DEC3 and
  08 §5.4.
- The feature's document comes with it, as a Spec until its last feature is ported (DOC5).

### 2.3 Rewrite

- The change writes the module to the contract of the section named in its row (02 for the
  adapters' glue, 03 for continuations, 04 for drawing, 05 for the event loop and the lock, 06 for
  the roots, 07 for the tour root and test support).
- No test of `main`'s module carries. Its tests are written with headers (07).
- A record of `main` the rewrite overturns is amended or replaced as 08 §5.4 says.

### 2.4 Drop

- The row says why and what holds the module's rule now, as 08 §8.2's table does for the lints.
  A dropped module that held a refusal is not dropped until something on `next` holds that
  refusal (§5.3).
- A decision whose only subject was the dropped module moves to `docs/history/decisions/` in the
  same change (DEC2, F268).

## 3. A module's commit

**Decision: a commit that copies, ports or rewrites names the files of `main` it carries, in its
body, in plain text: `From main at d89dde5de: <paths>`.** It also names the inventory file of a
port. It is a line of the body (G2), not a trailer (G3): no tool reads it. Reason: a reader of
`git log` can find where a module came from without the mapping; the mapping (§4) is what the
check reads. Turned down: a `Ported-from:` trailer, which G3 refuses, and naming nothing, which
leaves `git log -S` as the only way back to `main`.

## 4. The mapping

`docs/migration/mapping.toml` (Migration, 08 §4.1) holds one row per module of `main`: the plan's
§4, step 4. A row names a file or a directory of `main` at the tag; a directory row covers every
file under it that no more specific row names.

```toml
[[module]]
main = "faraday/faraday-core/src/vault/unlock.rs"   # file or directory at d89dde5de
fate = "port"                                       # copy | port | rewrite | drop
next = ["app/features/faraday-vault-unlock/src/update.rs",
        "app/features/faraday-vault-unlock/src/state.rs"]   # copy: the one path; drop: none
feature = ["vault-unlock"]                          # inventory slugs, for a port or a rewrite
contract = ""                                       # rewrite: the section, as "05 §4"
changed = []                                        # copy: files changed beyond a move, each with its cut
why = ""                                            # drop: why, and what holds its rule now
```

The rows are grouped by `main`'s crate. A copied crate is one directory row, with `changed`
naming its few edited files; a ported crate has a row per source file, since one file's logic may
land in several features.

**Decision: done is computed, never written.** A row is done when every path in its `next` exists
on `next` and holds no `todo!()`; a drop row is done once written. There is no status column.
Reason: a status kept by hand drifts from the code and is a work list in a document (DOC2), and
the skeleton's `todo!()` count is already the plan's measure. Turned down: a `done = true` field,
which a change can set without filling the stubs.

- **Rule MG4.** `python3 -I -B tools/migration.py check` holds the mapping: every path of
  `git ls-tree -r --name-only d89dde5de` falls under exactly one row; no row names a path absent
  at the tag; every `fate` is one of the four; a port or rewrite row names an inventory slug that
  exists, or a contract; a drop row has a `why`; a `next` path of a done row exists. It reports
  every failure in one run, and is a recipe of `lint-all`. Held by that script.
- **Rule MG5.** `tools/migration.py done` prints, for each inventory file and each crate of 09,
  the rows done and the `todo!()` left in their `next` paths. It is how progress is read; nothing
  is written back. Held by the script.

## 5. Porting one inventory feature file

### 5.1 What each section of the file becomes

The skeleton holds the crate of 09 §10 with the shape of 09 §4.4: its `State`, `Msg`, `Out`,
`Press`, `Model`, `Page` and `Refusal` exist and its bodies are `todo!()`. The routed `Out`
variants of 09 §4.4 are fixed; everything else below is written by the port, from the file.

| Inventory section | Becomes on `next` | Checked by |
|---|---|---|
| Screens and sheets | the crate's `Page` variants; each page's model and its fields (labels, values and actions, never a geometry); its `ANCHORS` | the feature's tour (07 §5), reviewed by a person beside `main`'s tour of the same screen at the same size, with the Material page named |
| Actions | a `Press` variant for each press; a `Msg` variant for each thing typed or brought in, or the part's message it goes to; a press that reaches another feature is an `Out` variant (Reach) | review: every action of the file has a variant, and none has two |
| Effects | the arms of `update`: asks, orders, jobs (`JOBS`), changes to owners, `Out`s; a worker's answer a `Msg` variant carrying its own context (03) | update tests for each effect a rule decides (TS3); journeys for a few (07) |
| State and lock | `State`'s private fields with their `AtLock` marks; `CARRY` (05 §7.4) | the lock's update test and the leak flows (07 §6) |
| Reach | `Out` variants and the parent composite's routing; what the feature reads through `Ctx` and `View` | the compiler (no edge between features, A5); the composite's routing tests |
| Rules and refusals | `Refusal` variants and the arms that refuse; a rule that can be a type is a type | one update test per refusal and per rule a plausible change breaks, with the header of 07 §3; a rule held by a type has no test (TS3) |
| Domain calls | the calls in `update` or in a job's body; costly ones take `Compute` | `costs-lint`; `lint-op-cost`; the cross-check through the domain function (TS20) |
| Fixtures | the files the update tests read (§5.4) | the mapping check |
| Fixes | parity rows (§5.3) | `tools/migration.py parity` |
| Doubts | settled by the specification's rule (05 for what a lock clears, 04 for what drawing may write), never by `main`'s behaviour | review; a doubt no section settles is settled there first (MG2) |

- **Kit models.** A kit model's fields (01 §6.5) are written by the change that ports its
  organism or template from its component inventory file (`session-pane.md`, `sheets-parts.md`,
  `ui-*.md`, `pane-container.md`, `flow-container.md`, `menu-pages.md`): what `main`'s drawing
  function took as arguments or read from `Faraday`, as labels, values and actions. A later
  feature that needs another field adds it in its own port.
- **The `Tool` enum.** Its variants (09 §4.1, the kernel's `handle`) are written in one change,
  the port of the Tools page: one per tile of `tools.md` and per row of `check-page.md` that opens
  a tool, as `main`'s `catalog::TILES` lists them. A tool not yet ported opens a page whose body
  is `todo!()`. Reason: the exhaustive `match` over `Tool` is the rule that every Check row has a
  tile (07 §5.4); an enum grown a tool at a time would let a row exist without its tile.

### 5.2 Review against main

- **Rule MG6.** A port's review reads the inventory file's Actions, Effects, State and lock, and
  Rules and refusals, and `main`'s modules of the rows it completes at the tag, and confirms each
  item has its place in the table above. Held by review; `.claude/agents/reviewer.md`'s checklist
  names it.

### 5.3 Refusals and the parity list

- **Rule MG7. What `main` refuses stays refused** (`main`'s `CLAUDE.md`, "Keys, cryptography").
  A refusal of the inventory file not held on `next` accepts more than `main` did, and needs a
  recorded decision (DEC3). Held by review.
- **The parity list** is the rows of `docs/migration/inventory/fixes.md` (817 bug fixes of
  `main`, each keyed by its commit), read as regressions `next` must not bring back. A row is
  accounted for in one of four ways:
  1. a test whose Regression line ends with the row's commit, as 07 §3 allows (`(main
     0d9ef7dd7)`);
  2. a type or lint that makes the break impossible, in `docs/migration/parity.toml` as `held =
     "compiler: <what>"` or `"lint: <name>"`;
  3. a shape that does not exist on `next` (a measure written while drawing, a field of
     `Faraday`), as `held = "gone: <why>"`;
  4. a check on the hardware or in a tour, as `held = "device: <tour or walk>"`.
- **Rule MG8.** `python3 -I -B tools/migration.py parity` lists every row of a feature whose
  crate holds no `todo!()` that is not accounted for, and every `parity.toml` entry naming a
  commit not in `fixes.md`. Held by the script.

### 5.4 Fixtures

The fixtures of `docs/migration/inventory/fixtures.md` are files of `main`, with copy rows in the
mapping. A fixture of a copied crate moves with the crate. A fixture of a ported or rewritten
crate goes to the `tests/` of the one crate on `next` that reads it, or to `tools/vectors/<its
folder>/` when more than one crate or the cross-check reads it: `faraday-core`'s
`tests/vectors/interop/` becomes `tools/vectors/interop/` (the wallet features and the
cross-check's `import` area read it), and the shells' fixtures (`edid`, `efibootmgr`, `udisks`,
`windows`) go with the adapter that parses them.

## 6. The test kit

`main`'s test data is made by `faraday-core`'s `testkit` module (`src/testkit.rs`, `plain.rs`,
`public.rs`), which `faraday/tools/testkit/` calls to write the test sticks, and which
`faraday-samples`' tests and the cross-check also call. Half of it makes data over domain types
(the ten test wallets of `kits()`, `TEST_SEEDS`, `test_words`, `savings()`, the unsigned and
cosigned spends, the MuSig2 rounds, the test vaults, `test_efi`); the other half drives
`Faraday` (`started`, `into_demo`, `backup_as`, `pause`, `new_key_on_length` and the like).

- **The data half is rewritten into `faraday-testkit`'s library**, over domain crates only, so
  that `faraday-app-test` and the domain crates' tests may name it (D4, T2). Where `main` signed
  through the session (`wallet::Session`, `wallet::Spend`), the test kit calls what the session
  calls: `faraday_bip::keys::MasterKey` over the test seeds and `faraday_psbt::sign` and
  `finalize`, as the cross-check's `review` and `sign` requests already do. Where it read a
  wallet, it calls `faraday_bip::policy::WalletPolicy`; `wallet::fp_text` is `Fingerprint`'s hex.
- **The driving half is dropped.** It is the support of `faraday-core`'s tests (TS4); `next`'s
  update tests use `faraday-app-test`, and its journeys the fixture adapters.

The four uses of `faraday-core` 09 §8 names, and where each now comes from:

| On `main` | Used by | On `next` |
|---|---|---|
| `wallet::Origin` | the test kit, only as an argument to `Session::add_wallet_confirmed` | not needed: the test kit adds no wallet to a session. `Origin` is the session's (`faraday-session`) |
| `testkit::backup_files` | the test kit's backup stick, made by `Faraday::new()` and `public_out`, the Back up step | the backup feature's own function that makes a wallet's public files, the one its `update` writes (`faraday-backup`, inventory `wallet-backup.md`), called by the writer of the sticks (below) |
| `picture::labelled_png` | the cross-check's `seedqr` area (`pictures`, `codex32qr`), and the backup's pictures | `faraday-draw`'s `picture` module, ported: it draws on `Canvas` in the baked fonts and writes `faraday_codec::png`; its one string (`public_pictures_part`) is passed in by its caller |
| `file_menu::sha256` | the cross-check's `hash` area (not the test kit) | `faraday_crypto::sha256_whole`, which `main`'s function called through `receipt::sha256_of`, and which the storage adapter calls (02 §8) |

**Decision: the writer of the test sticks is a tool crate of its own, `faraday-testkit-sticks`
(`tools/testkit-sticks/`), with normal edges to `faraday-testkit`, `faraday-backup` and the domain
crates the sticks need.** `faraday-testkit` keeps no application edge. Reason: every feature's
tests depend on `faraday-app-test`, which depends on `faraday-testkit`; an edge from the test kit
to a feature would be a cycle, and the backup stick must hold exactly what the backup feature
writes. Turned down: the test kit writing the backup's files with its own code, a second
implementation of the backup's output that would pass while the feature changed. Until the backup
feature is ported, the backup stick is not written.

## 7. The copied crates' exceptions

### 7.1 faraday-vault's clock and I/O names

09 §2.2 asks for the one file of `faraday-vault` that names a clock or I/O path. At the tag,
`git grep` of the io set (09 §1.4) over `faraday/faraday-vault/src/` finds one file:
`faraday/faraday-vault/src/contents/tree/toml_test.rs`, a `#[cfg(test)]` module (`tree.rs`
declares it) that reads toml-test's vendored cases from `tools/vectors/toml/testdata` with
`std::fs`. It is test code reading fixtures, not library I/O; nothing in the crate's library
names a clock or I/O path. Its relative path (`../../tools/vectors/toml`) holds from
`core/faraday-vault/` as from `faraday/faraday-vault/`. The crate's examples (`argon2_bench.rs`,
`large.rs`, `vectors.rs`) also read files, arguments and the clock: they are measuring and
vector-writing programs, copied as they are.

The same holds across the domain: at the tag, the io names appear in 49 test files, 15 examples
and 2 in-source test modules of the domain crates, and in their library code only where 01 §2.1
moves it out. Clippy's `disallowed_methods` is one lint, so allowing it for a test also allows the
mint and keys sets (09 §1.4). So D1 is read over library code only (01 §2.1, 09 §1.4).

### 7.2 faraday-samples' tests

**Rule MG9.** A copied test file of `faraday-samples` is kept when, after 09 §2.1's moves (the
samples' types to `faraday_samples::types`, `faraday_core::forms` and `strings` to
`faraday_forms` and `faraday_strings`, `testkit::compute` to `Compute::unchecked`, `testkit::kits`
and the other test data to `faraday-testkit`), it builds with domain and test-support
dev-dependencies alone. A file that needs `Faraday`, `faraday_shell_api::App`, `faraday_host` or
`wallet::Session` is dropped whole; nothing in it is edited to make it build but the one helper
named below. Held by the
copying change's build and review.

Measured at the tag (`tests/all/`):

| Kept | Dropped, and why |
|---|---|
| `published.rs`, `vectors.rs`, `keys.rs`, `main.rs` (its `mod` lines for the kept files), `common.rs` without `show`, `demo_app`, `strip` and `strip_offers`, which only dropped files use | `check_samples`, `chip_caret`, `core_wallet_sample`, `demo_strip`, `guide_key`, `hsm`, `kef_sample`, `load`, `ots_sample`, `passphrase`, `pi_panel`, `psbt`, `pull`, `signer_check`, `use_sample`: each drives `Faraday` or the Demo's host; `files.rs` and `wallets.rs`: each loads the samples into a `wallet::Session`; `handouts.rs`: it compares the samples with `faraday_core::testkit::demo_samples`, a copy kept for `faraday-core`'s tests |

What the dropped files held is the application's: what Practice loads and what the strip offers
are update tests of `faraday-strip` and of the features that load samples (inventory
`practice-demo.md`), written from that file and not from the dropped tests (TS4).

`Samples::handouts` needs the session and stays the application's; the kept `keys.rs` takes the
same list (the one-word words samples, in the order of `keys`) from a helper of its own over
`SAMPLES.keys`, the one edit MG9 allows a kept file.

The example `examples/regenerate.rs` comes with `faraday-testkit`, which holds the test data it
reads (the sample words, the entries file, the wallets and vaults it writes), with the same moves;
its two uses of `Session` (adding a wallet to sign a sample message) become the domain calls they
make (`faraday_intake::wallet::read_wallet` and `faraday_keys`' message signing). Until then
`faraday-samples`' crate documentation does not name it.

## 8. The crypto manifest while code is on main

`docs/crypto/actions.toml` comes with the domain (08 §4.4). At the tag it names `main`'s paths in
each action's `functions`, `code`, `checked.tests` and `screens`, 279 lines of them in
`faraday-core`; `docs/crypto/check.py` checks each against the tree and lists every
cryptographic function no action covers.

**Decision: a path of code not yet on `next` sits in the action's `[action.on_main]` table, and
moves out of it in the change that ports that code.**

```toml
[action.on_main]          # schema 6: what this action names on main and next does not yet hold
functions = ["faraday/faraday-core/src/wallet.rs::Session::sign"]
tests     = ["faraday/faraday-core/tests/all/spend.rs::a_held_key_is_refused"]
screens   = ["SpendBackup"]
```

- The change that copies `docs/crypto/` rewrites every copied crate's path to `next`'s
  (`faraday/faraday-keys/` to `core/faraday-keys/`) and moves every other path into `on_main`.
- **Rule MG10.** `check.py` checks a path in `on_main` at the tag and refuses it if it also
  resolves on `next`, or if its file's mapping row is done; it checks every other path on `next`.
  Coverage is over `next`'s tree only. Held by `check.py`.
- A test in `on_main.tests` never moves: `main`'s tests are not ported (TS4). The port deletes it
  and names in the action's `checked.tests` the test on `next` that holds the same invariant.
- `gen.py` writes a path in `on_main` as "on `main`" in the generated places, which resolve at the
  tag (DOC5, 08 §4.6).
- `lint-release` refuses a manifest with any `on_main` table. Held by `lint-release`.

Reason: the manifest stays whole and checked through the migration, and what is left is one table
per action. Turned down: dropping the actions whose code is unported until it is, which would
leave a ported function with no standard and no checks named.

## 9. The cross-check and Core

### 9.1 The areas of main, by what they call

On `main`, the cross-check areas `import`, `review` and `core_wallet` already call domain crates
only: `faraday_intake::wallet::read_wallet` and `faraday_bip::bsms` (`tools/crosscheck/src/import.rs`),
`faraday_psbt::inspect` (`review.rs`), `faraday_core_wallet::read_file` (`core_wallet.rs`). They
carry as they are; `import`'s path to the interop vectors follows §5.4 (07 §7.1). The
cross-check server's uses of `faraday-core` are three other requests:

| Request on `main` | Calls | On `next` (TS20) | The application's part, as an update test |
|---|---|---|---|
| `hash` (`file`) | `file_menu::sha256` | `faraday_crypto::sha256_whole` | none |
| `tx` (`decode`) | `decode::decode` over an empty regtest session: txid, wtxid, each output's address or its script's hex | `faraday-psbt`'s `transaction` module gains the view: the txid, wtxid and each output's address on a network (`bitcoin::Address::from_script`), refused over 21,000,000 BTC as `money_total` is | `faraday-decode`: which output is to a loaded wallet; the script's hex shown when it has no address |
| `seedqr` (`pictures`, `codex32qr`) | `picture::labelled_png` | `faraday-draw`'s `picture` (§6) | `faraday-backup`: which pictures a backup writes and their titles |

**Decision: the cross-check server may name `faraday-draw` for the picture requests alone.** The
picture a stick holds is drawn by the drawing base, a `no_std` crate with no application types;
the `seedqr` area checks that an outside reader decodes it to the code. Reason: a copy of the
drawing in the domain would be a second implementation of the picture. Turned down: dropping the
two requests, which would leave no outside check that a written picture reads back.

### 9.2 core-check and the interop example

`faraday/tools/core-check.py` checks Faraday against Bitcoin Core on regtest through
`faraday-core`'s `interop` example, which drives `Faraday`: `export` (the Back up step's public
files), `addresses`, `sign` (start the app, put the PSBT in the Inbox, load the wallet and seeds,
press Sign, read the Outbox) and `read` (what Files says a file is). It is the one check of
`main` that needs the application.

**Decision: the `interop` example is dropped, and `core-check.py` becomes the cross-check area
`regtest`, served by the cross-check server through the domain functions the features call.** No
step needs the application:

| core-check step | Domain function on `next` | The application's part |
|---|---|---|
| Core imports the wallet's Core file; its checksum of the descriptor text | `faraday_bip::core_import`, `WalletPolicy`'s checksummed descriptor, the writers the backup calls | `faraday-backup`: which files Back up writes (update test) |
| Core's first receive and change addresses | `WalletPolicy`'s derivation | none |
| Core funds and builds a spend; Faraday signs; Core accepts and mines it; two signers' PSBTs combine | `faraday_psbt::sign` and `finalize` over the test seeds, as the key store signs | `faraday-spend`: which inputs are signed, the refusals (update tests); the journey that signs a PSBT from the fixture stick (07) |
| Core's own `listdescriptors` read as its BIP-84 wallet | `faraday_intake::wallet::read_wallet` | none |
| `read` | `faraday_intake`'s file classification (below) and `read_wallet` | `faraday-files-area`: what a file is called (update test) |

`main`'s `faraday_core::wallet::{classify, classify_read, FileKind}` is a rule over a file's
bytes with no application state, and `op-cost` measures it (09 §8, edge to intake): it is ported
into `faraday-intake`. Reason for an area over keeping the script: the cross-check already runs
Core on regtest offline (`core_node.py`) and records its runs for `lint-release` (F197); a second
runner with its own example would drive the application from outside the port. Turned down:
keeping `interop` as an example of the device product, which a cross-check may not name (TS20).

## 10. What is never ported

- **`main`'s bugs.** A behaviour the inventory names as wrong or unsure (its Doubts, its Fixes'
  "what was broken") is not reproduced: `next` follows the specification's rule (a lock always
  restarts, 05). A difference
  from `main` that is one of its bugs is not a parity failure.
- **The history of fixes as code.** A fix's patch is not carried; the rule it restored is, as a
  parity row (§5.3). A branch of `main`'s code that exists only for a state `next`'s types cannot
  hold, or for a shape it does not have, is not ported.
- **`main`'s tests of ported and rewritten crates, and their assertions** (TS4), and the half of
  the test kit that drives `Faraday` (§6).
- **The shapes the plan overturns:** the `Faraday` struct, its glue and its ratchets; continuations
  in state; drawing that writes state or works things out; `lint-layout`, `lint-components`,
  `lint-check-tiles` (08 §8.2).
- **`main`'s documents of how it was built** (`docs/history/` but its decisions, and
  `STRUCTURE-PASS`, 08 §4.4), and the example that prints the early mock-ups' wallet
  (`faraday-core`'s `examples/wallet_2of3_demo.rs`).
- **Side channels on the offline device**, which are outside the security scope: a port neither
  adds nor carries a measure against them.

## 11. The order, as dependencies

What must exist before a change can be made. Within these, any order holds.

1. **The tag** (§1), before any check reads `main`'s tree.
2. **The skeleton** (09 §11), before any port fills a stub, with `lint-layers`, and the mapping
   with `tools/migration.py`, before any row is carried out.
3. **The domain**, before everything above it. The types each cut moves (`SecretText` and `ROOM`
   to `faraday-text`, `Refusal` and `Makes` to `faraday-keys`, `is_otpauth` to `faraday-codec`,
   `network_name` to `faraday-strings`, the blanks to `faraday-paper`) move in the change that
   copies the crates they leave, since a crate does not build with an edge half cut.
   `docs/crypto/` and the documents carried with the domain come in the same change (08 §4.4).
4. **The test kit's library** (§6), before any test that reads the test wallets.
5. **The port and the kit's copied crates**, before the kernel and the runtime; the organisms and
   templates a page draws on, before that page.
6. **The kernel, the presentation kernel and the parts**, before any feature; a part before the
   feature that embeds it; the step-flow machine before any flow.
7. **The shared owners** (09 §4.5), before a feature that changes or reads what they own.
8. **The runtime** (rewrite, 05), before a product crate; **the adapters' glue**, before the root
   that joins them.
9. **A product crate and its root**, before a journey through that product; the tour root,
   before a tour.
10. **The Tools page's port**, before a tool is reached from Check (§5.1); **the backup feature**,
    before the backup stick is written (§6).
11. **The cross-check server**, once the domain functions its requests name exist: the `tx` view
    and the picture with their changes (§9.1); `regtest` with file classification (§9.2).
12. **The cut-over**, last (§12).

## 12. The cut-over

The cut-over is one change on `next`, made when all of these hold: no `todo!()` in any crate
(08 §3.5); every mapping row done (MG4, MG5); every parity row accounted for (MG8); no `on_main`
table in the crypto manifest (MG10); and the parity checks on the hardware are passed (the plan's
§4, step 6).

**Decision: the cut-over change moves `docs/plan/` and `docs/migration/` to `docs/history/`,
with sections 09 and 10 of the specification (DOC4), and deletes `tools/migration.py` and its
recipe.** `docs/spec/README.md` keeps the two sections' lines, naming where they went;
`docs/README.md`'s lines change with them (DOC1). The documents, records and specification keep
citing `main`'s paths, which resolve at the tag (MG1). Then the branch `main` is set to `next`'s
tip; the two share no history, so this replaces the branch and does not merge it, and
`d89dde5de` stays reachable by its tag. Reason: the Migration documents are true only while
`main`'s code is being moved, and keeping them with the change that ends that keeps every
document's kind right. Turned down: moving them as each feature is ported, which splits the
inventory and the mapping that one script reads.

## 13. What holds each rule

| Rule | What it says, short | Held by |
|---|---|---|
| MG1 | `main` is read at its tag | `lint-docs`, `tools/migration.py`, `docs/crypto/check.py` |
| MG2 | a file's fate is recorded before it is carried out | `tools/migration.py check`; review |
| MG3 | a copy differs only by its named changes | review of `tools/migration.py diff` |
| MG4, MG5 | every file of `main` has one row; done is computed | `tools/migration.py check`, `done` |
| MG6 | a port's review reads the inventory file and `main`'s modules | review; the reviewer's checklist |
| MG7 | what `main` refuses stays refused | review; DEC3 |
| MG8 | every fix row is accounted for | `tools/migration.py parity` |
| MG9 | a samples test file is kept whole or dropped whole | the copying change's build; review |
| MG10 | unported code's paths in `on_main`, checked at the tag | `docs/crypto/check.py`; `lint-release` |

## Questions for later sections

None. The mapping's rows, the parity entries and each feature's variants are written by the
changes this section describes, not by the specification.
