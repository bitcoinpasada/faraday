//! A sealed vault goes back over its own file on a stick, found by salt
//! and length, and a write-back a pulled stick interrupted is finished on
//! the next visit (`docs/VAULT.md` §6).

use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};

/// A test's own directory standing for a stick; removed when it drops.
struct Stick(PathBuf);

impl Deref for Stick {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Stick {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A fresh directory standing for a stick.
fn stick(name: &str) -> Stick {
    let dir = std::env::temp_dir().join(format!("faraday-storage-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    Stick(dir)
}

/// A vault-shaped file: the magic, a 64 KiB slot length, a salt, and the
/// length that slot size gives.
fn vault(salt: u8, fill: u8) -> Vec<u8> {
    let mut v = vec![fill; 53 + 4 * (65_536 + 40)];
    v[0..4].copy_from_slice(b"OFVT");
    v[4] = 1;
    v[17..21].copy_from_slice(&65_536u32.to_le_bytes());
    v[21..53].fill(salt);
    v
}

#[test]
fn a_vault_goes_back_over_its_own_file() {
    let dir = stick("over");
    fs::write(dir.join("vault.ofv"), vault(1, 0)).unwrap();
    fs::write(dir.join("vault-2.ofv"), vault(2, 0)).unwrap();
    let id = dir.to_str().unwrap();
    // The second vault, sealed and offered as vault.ofv, still replaces
    // vault-2.ofv: the name is not the vault's identity.
    let wrote = faraday_storage::write_vault(id, "vault.ofv", &vault(2, 9)).unwrap();
    assert_eq!(wrote, "vault-2.ofv");
    assert_eq!(fs::read(dir.join("vault-2.ofv")).unwrap(), vault(2, 9));
    assert_eq!(fs::read(dir.join("vault.ofv")).unwrap(), vault(1, 0));
    // A vault no file on the stick has is written as a new one.
    let wrote = faraday_storage::write_vault(id, "vault.ofv", &vault(3, 0)).unwrap();
    assert_eq!(wrote, "vault-3.ofv");
}

#[test]
fn an_interrupted_write_back_is_finished() {
    let dir = stick("interrupted");
    // The original was deleted and the copy never renamed.
    fs::write(dir.join(".faraday-vault.ofv.part"), vault(1, 5)).unwrap();
    // A copy cut short beside its original.
    fs::write(dir.join("vault-2.ofv"), vault(2, 0)).unwrap();
    fs::write(dir.join(".faraday-vault-2.ofv.part"), &vault(2, 7)[..1000]).unwrap();
    faraday_storage::finish_vault_writes(&dir);
    assert_eq!(fs::read(dir.join("vault.ofv")).unwrap(), vault(1, 5));
    assert_eq!(fs::read(dir.join("vault-2.ofv")).unwrap(), vault(2, 0));
    assert!(!dir.join(".faraday-vault.ofv.part").exists());
    assert!(!dir.join(".faraday-vault-2.ofv.part").exists());
}
