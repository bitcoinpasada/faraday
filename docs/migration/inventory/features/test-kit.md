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

## Domain calls

- Every domain crate the app uses, through the app. `faraday/faraday-core/src/wallet.rs` `read_wallet` from `interop.rs read`.

## Fixtures

None.

## Fixes

None.

## Doubts

- The kit includes at compile time `tools/vectors/release-sums/release-key.asc`, `SHA256SUMS` and `SHA256SUMS.asc` (`faraday/faraday-core/src/testkit.rs:~1723-1737`); these paths are not in the kept fixture list.
- The budget tests name `tools/vectors/psbt/{signer-core-sent,signer-core-signed,bump-core-original}.psbt`, also not in the kept fixture list.
- `BOOTX64.EFI` is named but not included.
- Line numbers inside `faraday/faraday-core/src/testkit.rs` were approximate in the report.
- Not read: `faraday/faraday-core/src/testkit/plain.rs`, `faraday/faraday-core/src/testkit/public.rs` (beyond their names), `faraday/faraday-core/src/work/budget/signer.rs` (`mod compare`), most of `faraday/faraday-core/src/work/budget/shown.rs`.
- The interop vectors under `faraday/faraday-core/tests/vectors/interop/` are read by faraday-core's integration tests (`faraday/faraday-core/tests/all/interop.rs`, `faraday/faraday-core/tests/all/interop_outcomes.rs`), not by the kit or `faraday/faraday-core/examples/interop.rs`; they are listed with the features those tests cover.
- `faraday/faraday-core/src/work/budget/signer.rs` and `faraday/faraday-core/src/work/budget/shown.rs` are under `faraday/faraday-core/src/`.
