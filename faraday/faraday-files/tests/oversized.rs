//! A crafted stick: a hidden part file that claims a size no vault has is
//! removed without being read, and the vault write goes on.

use std::collections::BTreeMap;

use faraday_crypto::work::Compute;
use faraday_files::{OnTaken, Place, Zeroed, write_vault_over};

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-files' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

/// A stick whose listing can claim sizes the files do not have, and whose
/// reads refuse what is larger than asked, as a real stick's do.
#[derive(Default)]
struct Claimed {
    files: BTreeMap<String, (u64, Vec<u8>)>,
    oversized_reads: usize,
}

impl Place for Claimed {
    fn files(&mut self) -> Result<Vec<(String, u64)>, String> {
        Ok(self.files.iter().map(|(n, f)| (n.clone(), f.0)).collect())
    }
    fn read(&mut self, name: &str, max: u64) -> Result<Vec<u8>, String> {
        let f = self.files.get(name).ok_or("no such file")?;
        if f.0 > max {
            self.oversized_reads += 1;
            return Err("too large".into());
        }
        Ok(f.1.clone())
    }
    fn create(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        if self.files.contains_key(name) {
            return Err("taken".into());
        }
        self.files
            .insert(name.into(), (bytes.len() as u64, bytes.to_vec()));
        Ok(())
    }
    fn remove(&mut self, name: &str) -> Result<(), String> {
        self.files.remove(name).map(|_| ()).ok_or("none".into())
    }
    fn wipe(&mut self, name: &str) -> Result<faraday_files::Wiped, String> {
        self.remove(name).map(|()| faraday_files::Wiped::Zeroed)
    }
    fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        let f = self.files.remove(from).ok_or("none")?;
        self.files.insert(to.into(), f);
        Ok(())
    }
    fn space(&mut self) -> Result<(u64, u64), String> {
        Ok((0, 0))
    }
    fn zero_free(&mut self, _from: u32) -> Result<Zeroed, String> {
        Err("not here".into())
    }
    fn make_empty(&mut self) -> Result<(), String> {
        self.files.clear();
        Ok(())
    }
}

#[test]
fn a_part_file_of_a_hundred_megabytes_is_removed_unread_and_the_vault_is_written() {
    let mut stick = Claimed::default();
    stick
        .files
        .insert(".faraday-x.kdbx.part".into(), (100 << 20, Vec::new()));
    // A vault of the smallest size.
    let mut vault = faraday_codec::vault_file::SIGNATURE.to_vec();
    vault.extend_from_slice(&[0, 0, 4, 0, 0, 0, 0, 0, 0]);
    vault.extend_from_slice(&[1; 64]);
    vault.extend_from_slice(&[9; 32]);
    vault.extend_from_slice(&65_536u32.to_le_bytes());
    vault.resize(vault.len() + 65_536, 5);
    vault.extend_from_slice(&[9; 32]);
    vault.extend_from_slice(&0u32.to_le_bytes());
    let wrote = write_vault_over(
        &compute(),
        &mut stick,
        "x.kdbx",
        &vault,
        &[],
        true,
        false,
        OnTaken::Beside,
    )
    .unwrap();
    assert_eq!(wrote, "x.kdbx");
    assert!(!stick.files.contains_key(".faraday-x.kdbx.part"));
    assert_eq!(stick.files["x.kdbx"].1, vault);
    assert_eq!(stick.oversized_reads, 0, "the part file was never read");
}

#[test]
fn a_same_name_file_over_the_read_limit_is_asked_about_with_no_sha() {
    let mut stick = Claimed::default();
    stick.files.insert("x.kdbx".into(), (100 << 20, Vec::new()));
    let mut vault = faraday_codec::vault_file::SIGNATURE.to_vec();
    vault.extend_from_slice(&[0, 0, 4, 0, 0, 0, 0, 0, 0]);
    vault.extend_from_slice(&[1; 64]);
    vault.extend_from_slice(&[9; 32]);
    vault.extend_from_slice(&65_536u32.to_le_bytes());
    vault.resize(vault.len() + 65_536, 5);
    vault.extend_from_slice(&[9; 32]);
    vault.extend_from_slice(&0u32.to_le_bytes());
    let refused = write_vault_over(
        &compute(),
        &mut stick,
        "x.kdbx",
        &vault,
        &[],
        true,
        false,
        OnTaken::Ask,
    )
    .unwrap_err();
    assert_eq!(
        refused,
        faraday_files::WriteRefused::NameTaken {
            on_stick: "x.kdbx".into(),
            sha: None
        }
    );
    assert_eq!(stick.files.len(), 1, "nothing was written");
}
