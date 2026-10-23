# syn

**Purpose.** The Rust parser behind `tools/costs-lint`: it reads every
source file of the workspace into a syntax tree (`syn::parse_file`) and
visits it (`syn::visit`) to find who takes, mints and keeps the compute
token (`docs/COMPUTE.md` §8).

**Where it ships.** Nowhere. `costs-lint` is run by `just lint`
(`justfile`) and never runs on a device. No device, desktop or Starter
crate depends on `syn` directly; older and newer versions of it are in the
lock because procedural macros in third-party dependencies need them on the
build machine.

**Why this crate.** The lint needs a real parse: calls, method receivers,
function signatures and attributes, across files. A text search would miss
renamed imports and macros' arguments, and writing a Rust parser is out of
proportion. `syn` is the standard parser and is by the author of `quote`
and `proc-macro2`.

**Version and features.** `syn = { version = "2",
default-features = false, features = ["full", "visit", "parsing",
"printing"] }` in `tools/costs-lint/Cargo.toml`.

- `full` parses function bodies and items, not only derive input.
- `visit` is the read-only tree walker.
- `parsing` and `printing` are the parser and the `quote` output.
- `derive`, `clone-impls`, `extra-traits` and `proc-macro` are off.

**Review.** Trusted by publisher (dtolnay) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.

**Reopen when** `costs-lint` is replaced by a check that does not parse
Rust itself, or needs a syntax `syn` 2 cannot parse.
