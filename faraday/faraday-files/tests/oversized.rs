//! A crafted stick: a hidden part file that claims a size no vault has is
//! removed without being read, and the vault write goes on.

use std::collections::BTreeMap;

use faraday_files::{Place, Zeroed, write_vault_over};

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
    fn wipe(&mut self, name: &str) -> Result<(), String> {
        self.remove(name)
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
        .insert(".faraday-x.fvlt.part".into(), (100 << 20, Vec::new()));
    // A vault of slot size 0: 53 + 4 * 40 bytes.
    let mut vault = b"FVLT".to_vec();
    vault.resize(53 + 160, 0);
    let wrote = write_vault_over(&mut stick, "x.fvlt", &vault, true, false).unwrap();
    assert_eq!(wrote, "x.fvlt");
    assert!(!stick.files.contains_key(".faraday-x.fvlt.part"));
    assert_eq!(stick.files["x.fvlt"].1, vault);
    assert_eq!(stick.oversized_reads, 0, "the part file was never read");
}
