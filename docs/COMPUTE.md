# Compute: costly work takes a token

**Kind:** Reference
**Status:** Built

2026-10-21 · built (U4 to D3, 2026-10-21); owner approved the
approach 2026-10-20 (F69)

Every costly function in `core/` and in Faraday's crates takes a
`&Compute`, a token. Faraday mints one in two places: event code that
holds `&mut Faraday`, and the drawing memo's own fills. Drawing gets a
read-only `View`, so a screen that calls costly work does not compile.
The work counters, the budget tests (`faraday-core/src/work/budget/`,
`work/kept.rs`) and parse-once (`docs/SPEED.md` §3.3) all stay. The debug
guard was removed in D2, once the types and the lint enforced everything
it checked (§10).

Before the token, two values move into core: the fingerprint and the
checksum (§5). Each core change is its own commit. Nothing here
loosens a check.

## 1. The guarantee

- A function that draws can reach only `&View` or `&Faraday`. It cannot
  reach `Faraday::compute(&mut self)`, and it cannot name a token field.
- Every costly root (§2.3) requires `c: &Compute`. A costly call reached
  from drawing is a compile error, except through the named `View`
  accessors in `faraday-core/src/drawing/`: memo fills, kept values
  worked out the first time, and the owner's `Allowed` entries — exactly
  the guard's three exceptions.
- What the types cannot hold is held by `faraday/clippy.toml` (§7) and
  the costs lint (§8), both in `just lint`.

## 2. The token

### 2.1 Type (`core/faraday-crypto/src/work.rs`, beside `Work`)

```rust
pub struct Compute { count: fn(Work), _local: PhantomData<*const ()> }
impl Compute {
    /// A mint. An application calls this at its entry points only.
    pub fn new(count: fn(Work)) -> Compute;
    /// A token that counts nothing, for core's tests,
    /// fuzz targets and tools.
    #[cfg(feature = "unchecked")]
    pub fn unchecked() -> Compute;
    /// Notes one piece of work of kind `w`: a call through the pointer.
    pub fn count(&self, w: Work);
}
```

- `Compute` is not `Clone`, `Copy`, `Default`, `Send` or `Sync`, so it
  cannot be put in a `static`.
- A costly function takes `c: &Compute` as its first parameter after
  `self`, calls `c.count(kind)` on its first line, and its doc's first
  line says what it costs (CLAUDE.md).
- A gated type keeps no `FromStr` impl (`FromStr` cannot take a token);
  `"…".parse::<WalletPolicy>()` call sites become explicit calls.

### 2.2 The token carries the counter

It replaces S1's process-wide counter: `set_counter`, the `OnceLock` and
the `count` feature go, and `faraday-crypto` is `no_std` in every build.
Core's `Work` gains the kinds counted at source from now on: `WordsParse`,
`MasterKey`, `KeyDerive`, `Address`, `DescriptorParse`, `QrEncode`,
`FileHash`. The pixel kinds (`Blur`, `FullCanvas`) are drawing's own work
and are not gated: `faraday_draw::Canvas` gets `set_counter(&mut self,
fn(Work))`, set by Faraday on every canvas it receives. Faraday's own
counting is unchanged (`faraday-core/src/work.rs`: `count`, `Counts`,
`snapshot`, `since`, `KINDS`); its tokens carry `work::core_count`, which
maps the core kind with `of_core`. Faraday's own kinds (`PlanMap`,
`Picture`, `WalletMatch`, `PngDecode`, and `Kdf` in `faraday-vault`) still
call `crate::work::count`; the functions that count them take
`&Compute` as the gate.

### 2.3 What takes a token

The roots; every caller of a root takes the token too, and the compiler
finds them.

| Kind | Roots |
|---|---|
| `WordsParse` | `faraday_bip::bip39::Mnemonic::parse` |
| `Kdf` | `faraday_crypto::pbkdf2_hmac_sha256`, `pbkdf2_hmac_sha512`, `scrypt`; `faraday_backup::argon2id`; `faraday-vault`'s Argon2id. Through these: `Mnemonic::to_seed`, `to_seed_unchecked`, SLIP-39 recovery, aezeed, kdbx write and verify |
| `MasterKey` | `MasterKey::from_seed`, `from_seed_bytes`, `from_xpriv`, `decode`, `from_chain_code_and_key` (each works out the fingerprint, §5.1) |
| `KeyDerive` | `MasterKey::xpub`, `derive`, `account_xpub`, `multisig_account_xpub`, `with_multisig_account_secret`; `DerivedKey::to_xpub`; through these BIP-85 and silent payments derivations |
| `DescriptorParse` | `WalletPolicy::parse`, `from_parts`, `from_descriptor`, `parse_any`, `from_record`; through these the BSMS, Coldcard, multisig config and Core import readers, and `faraday_intake::wallet::read_wallet` |
| `Address` | `WalletPolicy::script_at`, `script_at_with`, `address_at`, `find_address` |
| `PsbtParse`, `PsbtInspect`, `Sign` | `faraday_psbt::Psbt::parse_bytes`, `parse_base64`; `inspect`; `sign::sign`, `message::sign` |
| `QrEncode` | `faraday_codec::qr::encode`, `encode_bounded`; `seedqr::encode_seedqr`, `encode_compact` |
| `QrDecode` | `faraday_codec::decode::decode_luma` |
| `FileHash` | new `faraday_crypto::hash::sha256_whole(c, bytes)`; Faraday's whole-file hashes move to it (`apk.rs`, `transfer.rs`, `lib.rs`, `gpg.rs`, `Item::sha`) |
| Faraday's own | `plan::map`, `plan::check` (`PlanMap`); picture/pdf makers (`Picture`); `Wallet::script_at` (`WalletMatch`); `faraday_files::qr_in_png` (`PngDecode`); `Key` and `Wallet` kept accessors (`KeyDerive`, `Address`) |

Not gated: `Canvas::frost`, `snapshot`, `snapshot_into`, `restore`,
`blend_from` (pixel work, counted per frame by the budgets);
`MasterKey::fingerprint`, `for_network`, `WalletPolicy::checksum`
(lookups after §5); `to_descriptor`, `to_text`, `Display` (text from parts
already parsed); `faraday_crypto::hash::sha256` on small fixed inputs;
`PolicyKey::parse` (inside gated parses); signature verification
(`message::verify`, BSMS `KeyRecord::verify`, events only today; a
`Verify` kind if a frame ever needs it).

## 3. Who mints

| Where | Constructor | Why |
|---|---|---|
| `faraday-core/src/compute.rs`: `Faraday::compute(&mut self) -> Compute` (pub) | `new(work::core_count)` | Event code: `event`, `press`, `act`, ticks, storage events, boot; the warm step before a frame (§4) |
| `faraday-core/src/drawing.rs`: `View::new` | `new(work::core_count)`, into a private field | Memo fills, kept values, `Allowed` |
| `faraday-storage/src/lib.rs`: `Pictures::ask` | `new(\|_\| {})` | The pictures' worker thread, which draws nothing; a token cannot cross threads, so the thread mints its own |
| `faraday-disk/src/main.rs`: `pictures` and `main` | `new(\|_\| {})` | The disk process's pictures thread and its serving loop, which draw nothing |
| `faraday/shells/snapshot/src/main.rs`: `main` | `new(\|_\| {})` | A tool |
| `faraday/tools/argon2-bench/src/main.rs`: `main` | `new(\|_\| {})` | A benchmark tool |
| `faraday-core/src/testkit.rs`: `testkit::compute()` (cfg `test`/`testkit`) | `new(work::core_count)` | Faraday's tests and the snapshot tool's sessions |
| `compute()` in `faraday-vault`'s `src/tests.rs` and `examples/vectors.rs`, `faraday-disk/tests/disks.rs`, `faraday-files/tests/picture.rs`, `faraday-keys/tests/operations.rs`, `faraday-qr/tests/formats.rs` and `interop.rs`, `faraday-scanner/src/lib.rs` (its unit tests), `faraday-starter/tests/all/{bitlocker,notes,template}.rs`, the Starter's `examples/snap.rs` | `new(\|_\| {})` | Tests and tools in crates that cannot reach faraday-core's test kit |
| Core tests and benches, `fuzz/`, `faraday/fuzz/` | `unchecked()` | No app guarantee |

The costs lint's `MINTS` (`tools/costs-lint/src/main.rs`) is this table,
file by file and function by function; a new mint is a line in both.

Core's non-test sources mint nothing (but `Compute::unchecked`, which is
itself made with `new`); they pass their callers' tokens through. The Starter's `phone_code` (a QR code each frame) is kept on its
state when the address is set (C3), so the Starter needs no mint while
drawing.

**Keeping Faraday off `unchecked`.** Since U5 (F68) no crate in the
normal dependency graph of a binary Faraday ships enables
`faraday-crypto/unchecked`: core's crates turn it on as dev-dependencies
only.
`tools/lint-unchecked.sh`, in `just lint`, fails if `cargo tree -e
features,normal -i faraday-crypto` over the shipped binaries, on any target,
shows it. Beside that, `faraday/clippy.toml` disallows
`Compute::unchecked` outright and `Compute::new` except under
`#[expect(clippy::disallowed_methods, reason = "a mint: …")]` at the sites
above (clippy reads the nearest `clippy.toml` above each manifest, so this
governs every crate under `faraday/` and no other); and the costs lint
rejects `unchecked` under `faraday/`, a mint outside its list,
and the `unchecked` feature in any `faraday/**/Cargo.toml`. `faraday/fuzz/`,
a workspace of its own with no app guarantee, is exempt from both:
its targets use `unchecked()` and its manifest turns the feature on.
An `#[expect(clippy::disallowed_methods)]` over `Compute::unchecked`
would silence clippy; only the costs lint rejects it, as it rejects
`unchecked` under `faraday/` whatever attribute is over it.

## 4. The drawing side

- `View<'a>` (`faraday-core/src/drawing.rs`, `pub`, `#[doc(hidden)]`):
  private fields `app: &'a Faraday`, `fill: Compute`, plus `after:
  RefCell<After>`; `Deref<Target = Faraday>`, no `DerefMut`.
- `After` collects what drawing writes today (scroll clamps, redraw
  requests from about 12 sites, `osk_track`'s result); `render` applies
  it after each pass.
- `render` is split: `App::frame` first calls `before_frame(&mut self)`,
  which mints and warms what was dropped since the last frame; `render`
  builds a `View` per pass, draws, applies `After`, and mints nothing.
- Accessors on `View` in `faraday-core/src/drawing/`: `memo.rs` (the 19
  memo fills), `kept.rs` (the ~15 kept values reached while drawing),
  `allowed.rs` (one named method per `Allowed` entry; today the three
  `SecretShown`). No generic closure hands out a token; a new exception
  is a new method, which the owner decides.
- Decisions use `*_now(&self, c: &Compute)` forms on `Faraday`; the memo
  exists only on `View`, so SPEED §2 rule 3 becomes a type rule.
- Screens that need no accessor may keep `app: &Faraday`; deref coercion
  lets `&View` call them.

## 5. U4 first: what core keeps

### 5.1 `MasterKey` keeps its fingerprint (`core/faraday-bip/src/keys.rs`)

`hold()` works out `fingerprint` once from the blinded context;
`for_network` copies it; `fingerprint()` returns it; `derive()` takes
`master_fingerprint` from the field (one EC multiplication saved per
derivation, in both apps). Faraday drops `wallet::Key.fp`. No output
changes; the BIP-32 vectors pass unchanged.

### 5.2 `WalletPolicy` keeps its checksum (`core/faraday-bip/src/policy.rs`)

Eager: every constructor sets `checksum: [u8; 8]`; `checksum(&self) ->
&str`. The derived `PartialEq` stays (the checksum is a function of the
other fields). Faraday drops `Wallet.checksum` and `set_policy`'s
recompute (it still drops kept addresses and scripts).

### 5.3 Hardened steps written `h` (owner-approved; DECISIONS F66)

Reading is unchanged: `'` or `h`, mixed or not. What is kept and written
is `h`: `parse_key` normalises `Origin.text`; the miniscript branch of
`to_descriptor` maps `'` to `h` (in a descriptor `'` is only a hardened
marker). Faraday drops `faraday_intake::wallet::hardened_h` and its
second parse (`read_wallet_as_written` folds into `read_wallet`) and the
`.replace('\'', "h")` in `faraday-core/src/wallet.rs`. Writers whose
format requires `'` convert at their own boundary (`backup.rs`,
`faraday-intake/src/electrum.rs`; the batch checks Caravan's `bip32Path`
and Coldcard's `Derivation:`). **BSMS:** `KeyRecord::signed_text`
re-renders `key.key_text()` and the signature is verified over it; a
received record keeps its key line as received and signs/verifies over
that line; a record made here writes `h`; the BIP-129 vectors in
`tools/vectors/bip129` must pass. Test: one wallet written with `'`, `h`
and mixed gives the same text and checksum, equal to the `h` form's
`descriptor_checksum`.

### 5.4 What changes for imported wallets

Wallets it made already write `h`. Wallets imported with `'` now export
`h` and show a different checksum. Faraday's stored records were already `h`. Tests whose
expectations change from `'` to `h`: `faraday-bip` (`policy.rs` units,
`coldcard.rs`, `multisig_config.rs`, `tests/all/{descriptor,bsms,musig,
recovery,silent}.rs`), `faraday-psbt` (`tests/all/multisig.rs`, `musig.rs`,
`tests/all/common/builder.rs`); reading tests keep their `'` inputs.

## 6. Batches

Each is one implementer; `just` green at the end of each; each core
change its own commit.

| Batch | Content | After | Alongside | Model |
|---|---|---|---|---|
| U4a | §5.1 | — | U4b | Sonnet |
| U4b | §5.2, then §5.3–5.4 (two commits), DECISIONS F66 | — | U4a | Opus |
| A0 | `View`, `After`, `before_frame`; the 11 `&mut Faraday` draw roots take `&View`; `osk_track` leaves drawing; the guard stays | U4 | — | Opus |
| A1 | Memo, kept and allowed accessors on `View`, beside the old forms; decisions switch to `*_now` | A0 | — | Opus |
| B1 | `screens/` | A1 | B2–B6, C0 | Sonnet |
| B2 | `vault/screen.rs`, `vault/load_screen.rs`, `glance/sheet.rs`, `glance.rs` | A1 | ″ | Sonnet |
| B3 | `backup_flow/screen.rs`, `backup_flow/wallet_screen.rs`, `board/screen.rs`, `plan/cards.rs`, `backups/screen.rs` | A1 | ″ | Sonnet |
| B4 | `family/screen.rs`, `family/text.rs`, `keygen/screen.rs`, `seeds/screen.rs`, `codes/screen.rs`, `bip85/screen.rs` | A1 | ″ | Sonnet |
| B5 | `compact.rs`, `compact/screen.rs`, `boot_import/screen.rs`, `in_progress.rs`, `stick/pills.rs`, `stick/wipe.rs`, `upgrade/screen.rs`, `transfer/screen.rs`, `faraday/faraday-core/src/tools/screen.rs` | A1 | ″ | Sonnet |
| B6 | the rest (`explore_screen`, `silent_screen`, `vanity_screen`, `lightning_screen`, `wordlist_screen`, `file_menu`, `edit`, …) | A1 | ″ | Sonnet |
| C0 | `Compute` and `unchecked`; `Faraday::compute`; `View`'s token; the mint sites; `faraday/clippy.toml` with the two mint entries; the global counter stays | A1 | B1–B6 | Opus |
| C1 | Gate `WordsParse`, `Kdf`, `MasterKey`, `KeyDerive`; their counts move to `c.count` | B, C0 | — | Sonnet |
| C2 | Gate `DescriptorParse`, `Address`, `WalletMatch`, `PlanMap`; `faraday-intake`'s miniscript call moves into `faraday-bip` | C1 | D1 drafting | Sonnet |
| C3 | Gate PSBT, QR, `PngDecode`, `Picture`, `FileHash`; the Starter's QR kept on its state; remove `set_counter`, the `OnceLock`, the `count` feature; `Canvas::set_counter`; drop the old memo forms | C2 | D1 drafting | Sonnet |
| D1 | Costs lint (§8), compile_fail doctests (§9), full `faraday/clippy.toml` (§7), in `just lint` | C3 | — | Opus |
| D2 | Remove the guard (§10) | D1 | D3 | Sonnet |
| D3 | Docs (§11) | D2 | — | Sonnet |

C1–C3 run in sequence (they share ~700 call sites). Each C batch updates
`fuzz/` and `faraday/fuzz/` for what it gates. When a count moves to its
source the batch re-derives `moments.rs` and `presses.rs` and says why
each number moved; no frame budget may rise from none. The guard stayed
active through every C batch.

Built (2026-10-21):

- **D1.** The costs lint, the compile_fail doctests, the full
  `faraday/clippy.toml` with `tools/clippy.py`, and new budgets (each
  kept accessor, allocations and lines a frame, each typed character,
  the Pi's panel, a long session's kept lists; `docs/SPEED.md` §8.3).
  Found: `View::item_hash` hashed a file on every read when it had no
  hash from intake; three sheets stretched a key on every character
  typed (now after a 400 ms pause, `docs/SPEED.md` §9 item 2); four
  kept lists had no bound.
- **D2.** The debug guard removed (§10); the token, the costs lint, the
  doctests and the budget tests hold the rule.
- **D3.** These docs: `docs/SPEED.md` §3.1 to §3.3, §8, §9; CLAUDE.md;
  DECISIONS F66 and F69. Found: §3, §7, §9 and
  §12 corrected to what D1 built.

## 7. `faraday/clippy.toml`

Third-party paths only. `disallowed-methods`: `Compute::new` (allowed
only by `#[expect]` at §3's sites), `Compute::unchecked`,
`bitcoin::bip32::Xpriv::new_master`, `Xpriv::derive_priv`,
`Xpriv::fingerprint`, `Xpub::from_priv`, `Xpub::derive_pub`,
`secp256k1::PublicKey::from_secret_key` (`lightning.rs` moves to a gated
`faraday-bip` function), `bitcoin::psbt::Psbt::deserialize`.
`disallowed-types`: `miniscript::Descriptor` (`testkit.rs` and `examples/`
under `#[expect]`). Hashing engines are not listed (risk 5). Paths
resolve through the `faraday_bip::bitcoin`/`miniscript` re-exports.

A path that names nothing (mistyped, or moved in a dependency) does not
fail `-D warnings`: clippy prints a warning about its configuration and
exits 0, and the entry guards nothing. `tools/clippy.py` runs clippy and
fails when any message points into a `clippy.toml`; `just lint` runs
clippy through it. The `Compute::unchecked` entry carries
`allow-invalid = true`: the crates clippy checks under `faraday/` do not
turn on `unchecked`, so the path names nothing in them, and the entry
still holds wherever the feature is on.

## 8. The costs lint

`tools/costs-lint`, a workspace binary on `syn` 2 (`full`, `visit`), run
by `just lint`; walks `core/*/src` and `faraday/**/src`, skipping
`#[cfg(test)]`; fails on:

1. a function with a `&Compute` parameter whose doc's first line does not
   state its cost; a function that calls `.count(` and takes no
   `&Compute`;
2. `Compute::new` outside §3's list (tests and tools included),
   `Compute::unchecked` anywhere under `faraday/` but `faraday/fuzz/`,
   either constructor in core's non-test sources (but `unchecked`'s own
   body), the `unchecked` feature in a `faraday/**/Cargo.toml` but the
   fuzz crate's;
3. a field, `static` or `thread_local` of type `Compute` other than
   `View.fill`;
4. a `.compute()` call or `Compute::` path inside `render*` (but
   `before_frame`) or any function taking `&View` or `&mut Ui`.

## 9. compile_fail doctests

`core/faraday-crypto/src/work.rs`, on `Compute`, beside an example that
compiles: `c.clone()` (E0599), `Compute: Send` (E0277), `Compute:
Default` (E0277). `faraday-core/src/drawing.rs`, on `View`:
`v.compute()` on `&View` (E0596), `app.compute()` on `&Faraday`
(E0596), `WalletPolicy::parse(t)` without a token (E0061), `&v.fill`
(E0616); twins that compile: `fn event(app: &mut Faraday, text: &str) {
let c = app.compute(); WalletPolicy::parse(&c, text); }` and a draw
reading `v.screen`.

Rustdoc on this toolchain does not check the error code a
`compile_fail` example names: an example that fails for another reason
(a typo, a path moved) still passes. The compiling twin beside each,
differing in the one thing refused, is what shows the refusal is the
one meant; the codes are written for the reader.

## 10. When the debug guard is removed

In D2, the commit after D1, green on top of C3: every counted kind but
the two pixel kinds requires `&Compute`, the only token drawing can
reach is `View`'s (used only by the guard's own three exceptions), and the
lint and doctests hold the rest. Removed from `faraday-core/src/work.rs`:
`Frame`, `DRAWING`, `FILLING`, `filling`, `keeping`, `allowed`, `callers`,
the panic in `count`, and `PIXELS` if nothing reads it; `render`'s
`work::Frame::begin()`. The `Allowed` enum moves to `drawing/allowed.rs`
as the owner's list. Kept: `count`, `Counts`, `snapshot`, `since`,
`KINDS`, `of_core`, `core_count`, `work/budget/`, `work/kept.rs`, and
everything parse-once keeps.

## 11. Docs

`docs/SPEED.md` §3.1–§3.2 (core keeps fingerprint and checksum), §3.3
("The guard" becomes "The token"), §8.2 (new sources); CLAUDE.md's
"Drawing reads" bullet (the token replaces the guard sentence);
`docs/DECISIONS.md` F66 (`h`, and
the token).

## 12. Risks

1. Merge conflicts: ~700 call sites, mostly tests; C1–C3 in sequence, B in
   parallel by disjoint files.
2. BSMS verification (§5.3), gated by the BIP-129 vectors.
3. Checksums of `'`-written wallets change (§5.4);
   formats needing `'` convert at their writer.
4. `unchecked` was compiled into Faraday until U5 (F68);
   `tools/lint-unchecked.sh` keeps it out.
5. Residual: a new whole-file hash written with `bitcoin::hashes` in a
   screen is not caught (the guard did not catch it either).
6. `View` grows to ~37 thin accessors in `drawing/`, each saying what it
   reads.
7. Count tables shift as counts move to their source; batches explain
   each change.
8. Rustdoc on this toolchain does not check pinned error codes (found
   in D1). The compiling twin beside each `compile_fail` doctest is the
   real guard: it catches a doctest failing for the wrong reason (§9).

## 13. Build time

Runtime nil (one pointer; one indirect call inside microsecond work).
Costs lint ~10 s clean, ~1 s after. Doctests: ~15 s more for `cargo test
--doc`. Removing the guard makes tests marginally faster.

## 14. Estimate

U4a 0.5, U4b 1, A0 1, A1 1, B1–B6 3 (about 1 elapsed), C0 0.5, C1 1, C2 1,
C3 1.5, D1 1, D2+D3 0.5 implementer days: about 12 of work, about 8
elapsed with the parallel batches.
