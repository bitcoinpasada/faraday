//! The integration tests of faraday-storage, one module per subject.

mod boxes;
mod foreign_files;
mod pictures;
mod settings_write;
mod stick_folders;
mod transfer;
mod vault_write;
mod vm_stat;

/// A token for the listings the tests make.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-storage's tests, which draw nothing (docs/COMPUTE.md §3)"
)]
pub(crate) fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}
