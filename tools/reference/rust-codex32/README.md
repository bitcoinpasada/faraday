# rust-codex32, vendored

The Rust reference implementation of codex32 (BIP 93), copied unchanged
into `codex32/`, and a small program of its own beside it that answers the
crypto cross-check (`tools/crosscheck/area_codex32.py`, F79) in rust-codex32's
words. It is a reference, never shipped: the crate is not a member of the
repository's workspace and sits outside `cargo vet`, its program is built
offline by `tools/crosscheck/run.sh` (`cargo build --frozen`, into
`out/crosscheck/rust-codex32-ref`), and `Cargo.lock` is committed.

Source: `https://github.com/apoelstra/rust-codex32`, the head of its
default branch, commit `3943cb52b2343333386fac0cd38b8ae5a4d6de02`
(26 February 2024, "Merge apoelstra/rust-codex32#1: fix
Codex32String::from_unchecksummed_string"), crate version 0.1.0 (the
repository has no tag). `LICENSE` is CC0 1.0 and `Cargo.toml` names the
same; the crate has no dependency. Fetched with `git clone` on
2026-11-03. The commit is not signed: what pins the copy is the commit
hash and the digests below.

Copied: `Cargo.toml`, `LICENSE`, `src/lib.rs`, `src/checksum.rs`,
`src/field.rs` and `src/bin/correction-table.rs` (which `Cargo.toml` names
as a binary; it is not built here). Left out: `src/bin/list.txt` (30 MB of
data that only the binary reads), `Cargo.lock`, `clippy.toml`, `contrib/`
and the upstream `README.md`.

`src/main.rs` and `Cargo.toml` and `Cargo.lock` at this folder's top are
written here: a program that reads a request a line and answers a line,
for `parse` (`Codex32String::from_string`), `from_seed`, `make`
(`from_unchecksummed_string`) and `fresh` (`from_unchecksummed_string`,
then `interpolate_at` at each derived index and at `s`). What the crate
decides (the threshold of 0 or 2 to 9, the share index `s` of a threshold
of 0, the lengths) the program passes on as the crate answers it. The
protocol is at the top of `src/main.rs`.

SHA-256 of every file, so a change to the vendored copy is visible. The
same digests are in `tools/crosscheck/references.sha256`, which the
cross-check checks before it runs anything, refusing to run on a mismatch.

| File | SHA-256 |
|---|---|
| `Cargo.toml` | `2547477abcb1a2ffe275d4cf546412f5b2de83250ff855936f1de13a92939ae1` |
| `Cargo.lock` | `ccc4e8ab95034baf809cff73617302c48a04046b26d5d5f8d0edcbe5eae6d864` |
| `src/main.rs` | `5c562db69954d801dfdd2884c0d119723b0c9c865c2e715d6953cab25db588ae` |
| `codex32/Cargo.toml` | `037fd241be928746c7d8e2e98238aee78e3106f78a16fb439fab1253c068e213` |
| `codex32/LICENSE` | `7179683e8000e6bdc9bbc60d85edf0a4ac8e76f951857f54fcb775d5886f1309` |
| `codex32/src/lib.rs` | `978f3aa640c4ace5a8b4cc54028b7896e5328697ca869ef38b4869c16d7dcf9b` |
| `codex32/src/checksum.rs` | `fa9adffeca0169962aea76fdd7765a7d3b0d01fea5816dc200ed9821e9a0b30c` |
| `codex32/src/field.rs` | `0d60384d83f462d1521a245136b4b5d2420140bc6a16ec964ead35c9b3b35e73` |
| `codex32/src/bin/correction-table.rs` | `67f661b061bde6348b6345c845f03f932e1ec321a7c336bf8b73d040a90b7fe8` |
