//! The crypto inventory (`docs/CRYPTO-LEARN.md` §4): see
//! [`costs_lint::inventory`].

use std::process::ExitCode;

fn main() -> ExitCode {
    costs_lint::inventory::main()
}
