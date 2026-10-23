# quote

**Purpose.** Turning `syn` syntax nodes back into text in
`tools/costs-lint` (`docs/COMPUTE.md` §8), so the lint can print the
offending expression or signature beside a finding.

**Where it ships.** Nowhere. It is used by a tool run by `just lint` that
never runs on a device. No device, desktop or Starter crate depends on it
directly.

**Why this crate.** It is the printing counterpart of `syn` (the `printing`
feature of `syn` is built on it) and the companion of `proc-macro2`; the
three are used together and have no alternatives.

**Version and features.** `quote = { version = "1",
default-features = false }` in `tools/costs-lint/Cargo.toml`: no
`proc-macro` feature, since the lint is an ordinary binary.

**Review.** Covered by audits imported from other organisations (`supply-chain/imports.lock`); not read here.

**Reopen when** `costs-lint` stops printing Rust source fragments, or goes
with `syn`.
