//! Writing over a file on a stick, after the person chose Replace: the
//! new bytes end up under the old name and nothing else is left behind;
//! a write whose read-back differs leaves the old file as it was.

use std::collections::BTreeMap;

use faraday_files::{Place, Zeroed, write_any, write_replacing};

/// A stick's top level in memory. `garble` makes every read of a hidden
/// temp file come back different from what was written, as a failing
/// stick would.
#[derive(Default)]
struct Memory {
    files: BTreeMap<String, Vec<u8>>,
    garble: bool,
}

impl Place for Memory {
    fn files(&mut self) -> Result<Vec<(String, u64)>, String> {
        Ok(self
            .files
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect())
    }
    fn read(&mut self, name: &str, _max: u64) -> Result<Vec<u8>, String> {
        let mut b = self.files.get(name).cloned().ok_or("no such file")?;
        if self.garble && name.starts_with('.') {
            b.push(0);
        }
        Ok(b)
    }
    fn create(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        if self.files.contains_key(name) {
            return Err("taken".into());
        }
        self.files.insert(name.into(), bytes.to_vec());
        Ok(())
    }
    fn remove(&mut self, name: &str) -> Result<(), String> {
        self.files
            .remove(name)
            .map(|_| ())
            .ok_or("no such file".into())
    }
    fn wipe(&mut self, name: &str) -> Result<(), String> {
        self.remove(name)
    }
    fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        if self.files.contains_key(to) {
            return Err("taken".into());
        }
        let b = self.files.remove(from).ok_or("no such file")?;
        self.files.insert(to.into(), b);
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

fn stick(files: &[(&str, &[u8])]) -> Memory {
    Memory {
        files: files
            .iter()
            .map(|(n, b)| (n.to_string(), b.to_vec()))
            .collect(),
        garble: false,
    }
}

#[test]
fn replacing_writes_the_new_bytes_under_the_old_name_and_leaves_no_temp() {
    let mut s = stick(&[("Wallet.json", b"old"), ("other.txt", b"x")]);
    // The name matches whatever its case on the stick.
    let wrote = write_replacing(&mut s, "wallet.json", b"new bytes").unwrap();
    assert_eq!(wrote, "Wallet.json");
    let names: Vec<&String> = s.files.keys().collect();
    assert_eq!(names, ["Wallet.json", "other.txt"], "no temp, no copy");
    assert_eq!(s.files["Wallet.json"], b"new bytes");
}

#[test]
fn a_failed_read_back_leaves_the_old_file() {
    let mut s = stick(&[("wallet.json", b"old")]);
    s.garble = true;
    assert!(write_replacing(&mut s, "wallet.json", b"new").is_err());
    let names: Vec<&String> = s.files.keys().collect();
    assert_eq!(names, ["wallet.json"], "no temp left");
    assert_eq!(s.files["wallet.json"], b"old");
}

#[test]
fn keep_both_writes_beside_the_old_file() {
    let mut s = stick(&[("wallet.json", b"old")]);
    let wrote = write_any(&mut s, "wallet.json", b"new", false).unwrap();
    assert_eq!(wrote, "wallet-2.json");
    assert_eq!(s.files["wallet.json"], b"old");
    assert_eq!(s.files["wallet-2.json"], b"new");
}

#[test]
fn a_vault_is_never_written_over_another_vault() {
    // Two vaults of different salts: the old one stays whatever the
    // write asks.
    let vault = |salt: u8| {
        let mut v = b"FVLT".to_vec();
        v.extend_from_slice(&[0u8; 13]);
        v.extend_from_slice(&65_536u32.to_le_bytes());
        v.extend_from_slice(&[salt; 32]);
        v.resize(faraday_codec::vault_file::file_len(65_536), 0);
        v
    };
    let old = vault(1);
    let mut s = stick(&[("vault.fvlt", &old)]);
    let wrote = write_any(&mut s, "vault.fvlt", &vault(2), true).unwrap();
    assert_eq!(wrote, "vault-2.fvlt");
    assert_eq!(s.files["vault.fvlt"], old);
}

#[test]
fn a_replacing_write_never_removes_a_vault() {
    // Whatever name's case or contents the write carries, a vault on the
    // stick stays, and the new file goes beside it.
    let old = b"FVLT vault bytes".to_vec();
    for (name, bytes) in [
        ("VAULT.FVLT", b"FVLT other".as_slice()),
        ("vault.fvlt", b"not a vault".as_slice()),
    ] {
        let mut s = stick(&[("vault.fvlt", &old)]);
        let wrote = write_any(&mut s, name, bytes, true).unwrap();
        assert_ne!(wrote.to_ascii_lowercase(), "vault.fvlt", "{name}");
        assert_eq!(s.files["vault.fvlt"], old, "{name}");
    }
}
