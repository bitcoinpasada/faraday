# Test kit

The test kit: a fixture and driver library for tests, snapshot tours and tools, built only with the `testkit` feature and kept out of shipped binaries. It also holds the leak tests (plant secret forms, check the drawn ink), the frame and costly-work budget tests, and two examples. Nobody using the device or a wallet reaches it.

## Screens and sheets

None. It drives the real screens through presses.

## Actions

- Drivers: `backup_as` (presses the backup board to a preset), typing helpers, worker holds, fault injection (`vault::fail_next_*`).
- Leaks (`faraday/faraday-core/src/leaks.rs`, `testkit` only): plants secret forms and checks the ink drawn; the hook `crate::leaks::page_drawn` is called from the screen frame (`faraday/faraday-core/src/screens/mod.rs:~447-448`).
- Worker test hooks: `HELD`, `ON_START`, `wait_answer` (120 s).

## Effects

- Builds a test stick in memory (`files_t`): wallets, `-wallet.json`, `-multisig-setup.txt`, unsigned and cosigner-signed PSBTs, an Inheritance recovery PSBT (sequence 52_560), MuSig2 files, a vault, `vault-entries.txt`, a `BOOTX64.EFI` built in code (`test_efi`), `SHA256SUMS` and `SHA256SUMS.asc`, Savings shares, QR PNGs, seed-word and SeedQR PNGs, xpub files, `spending-message.txt`.
- Examples:
  - `faraday/faraday-core/examples/interop.rs`: the driver for `faraday/tools/core-check.py`. Commands: `kits`, `export <kit> <dir>`, `addresses <kit> <n>`, `sign <kit> <seeds> <in.psbt> <dir>`, `read <file>`.
  - `faraday/faraday-core/examples/wallet_2of3_demo.rs`: prints a 2-of-3 worked example from fixed words; writes no files.

## State and lock

- Test-only. Vaults: `test_vault` with passphrase "test vault"; `seeds_vault` and the backup vault with passphrase "a"; KeePass helpers with an 8 MiB cost.
- Demo samples (`demo_samples`, `faraday/faraday-core/src/testkit.rs:~195`): 42 fingerprints from 21 word lists, the strings leaked once per process.
- Test seeds are public dummy words ("bacon", "zebra", "summer" x24). Randomness is deterministic (fixed seeds, `[id; 16]`).
- `made_clean` and `holds_nothing` check that a lock left nothing held.

## Reach

- Used by the tests of every feature, by the snapshot tours (`faraday/shells/snapshot/src/tours/`) and by the cross-check tools (`tooling`).
- The budget tests (`drawing-compute`): `faraday/faraday-core/src/work.rs:~389-390` is `#[cfg(test)] pub(crate) mod budget;`, so `faraday/faraday-core/src/work/budget.rs`, `work/budget/*.rs` and `faraday/faraday-core/src/work/kept.rs` are test code (`FRAME_CEILING`, `FRAME_CEILING_PI`, desktop and Pi panels).
- `faraday/faraday-core/src/testkit/plain.rs` and `faraday/faraday-core/src/testkit/public.rs`.

## Rules and refusals

- `leaks` and `testkit` exist only with the `testkit` feature (`faraday/faraday-core/src/lib.rs:~257-268`). `tools/lint-keys-tests.sh` keeps the kit out of shipped binaries (per its comment).
- Rules the kit encodes in panics and asserts:
  - A key without an origin is refused (`unsigned_with_xpubs`).
  - A public sample is refused on Mainnet.
  - With no secret held, a stick write may start (`inbox_write_started`).
  - `use_network` refuses a second, different network in one test.
- `faraday/faraday-core/examples/interop.rs` refuses: an unknown test wallet id, an address count that is not a number, a wallet that does not arrive in the Inbox, a PSBT that does not read, a spend that does not open, nothing signed, nothing in the Outbox.
- Test-only items in production files: `back_offered` under `testkit` (`faraday/faraday-core/src/trail.rs:~570-574`); `edit_answers` and others under `cfg(any(test, feature = "testkit"))`.
- `testkit::test_efi()` builds the test boot program in code (a 1024-byte PE32+ file, one `.text` section holding a return, `faraday/faraday-core/src/testkit.rs:1742-1775`); it equals `tools/vectors/release-sums/BOOTX64.EFI` byte for byte, and the kit's file list names it `BOOTX64.EFI` beside the signed sums (`faraday/faraday-core/src/testkit.rs:630-634`).
- The kit's public fixtures are plain text (`String`) and name no secret type; seeds and what is made from them stay in the parent module (`faraday/faraday-core/src/testkit/plain.rs:1-8`); `KindText` allows the disallowed key types only because the kit is in no shipped binary (`faraday/faraday-core/src/testkit/public.rs:5-8`). The shown-secret budget tests hold that a shown secret is worked out once per press, frames work out nothing, and hiding, leaving, the idle warning and a lock drop it (`faraday/faraday-core/src/work/budget/shown.rs:1-856`); the signer-check and compare tests hold that only the press completing the second card does costly work (`faraday/faraday-core/src/work/budget/signer.rs:1-217`).

## Domain calls

- Every domain crate the app uses, through the app. `faraday/faraday-core/src/wallet.rs` `read_wallet` from `interop.rs read`.

## Fixtures

- `tools/vectors/release-sums/release-key.asc`, `SHA256SUMS` and `SHA256SUMS.asc`, included at compile time (`faraday/faraday-core/src/testkit.rs:1717-1737`) as the test release key, the signed sums and the signature; the same folder holds `BOOTX64.EFI`, `other-key.asc`, the other signature files, `variants/`, `sequoia/` and `faraday-0.1.7/`, read by `faraday/faraday-pgp/tests/verify.rs`, `faraday/faraday-pgp/tests/verify_refusals.rs`, `faraday/faraday-sb/tests/release.rs` and `faraday/faraday-core/tests/all/secure_boot_sign.rs`.
- The ten test wallets of `kits()` (`faraday/faraday-core/src/testkit/plain.rs:75-177`), as id, name, descriptor, cosigner seed, fee in sat: savings Savings `wsh(sortedmulti(2,3 keys))` at `m/48'/1'/0'/2'` 1, 2140; spending Spending `wpkh` at `m/84'/1'/0'` none, 1410; taproot Taproot `tr` at `m/86'/1'/0'` none, 1540; nested Nested SegWit `sh(wpkh)` at `m/49'/1'/0'` none, 1660; legacy Legacy `pkh` at `m/44'/1'/0'` none, 2260; vault-nested Nested multisig `sh(wsh(sortedmulti(2,…)))` at `m/48'/1'/0'/1'` 1, 2560; vault-legacy Legacy multisig `sh(sortedmulti(2,…))` at `m/45'/1'/0'` 2, 3380; taproot-multisig Taproot multisig `tr(NUMS,sortedmulti_a(2,…))` at `m/48'/1'/0'/3'` 1, 1820; inheritance Inheritance `wsh(or_d(pk,and_v(v:pkh,older(52560))))` at `m/48'/1'/1'/2'` none, 1980; tree Taproot tree `tr(key,{and_v(v:pk,older(4320)),pk})` at `m/86'/1'/1'` none, 1540. Each is written to the test stick as `<id>-wallet.txt` (`# <name>: test wallet over the dummy test seeds`, then the descriptor).

## Fixes

None.

## Doubts

- Line numbers inside `faraday/faraday-core/src/testkit.rs` were approximate in the report.
- The interop vectors under `faraday/faraday-core/tests/vectors/interop/` are read by faraday-core's integration tests (`faraday/faraday-core/tests/all/interop.rs`, `faraday/faraday-core/tests/all/interop_outcomes.rs`), not by the kit or `faraday/faraday-core/examples/interop.rs`; they are listed with the features those tests cover.
- `faraday/faraday-core/src/work/budget/signer.rs` and `faraday/faraday-core/src/work/budget/shown.rs` are under `faraday/faraday-core/src/`.
