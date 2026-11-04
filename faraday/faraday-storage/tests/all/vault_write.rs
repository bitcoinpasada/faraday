//! A sealed vault goes back over its own file on a stick, found by the
//! SHA-256 of the copies the app has seen, and a write-back a pulled stick interrupted is finished on
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

/// A whole KDBX 4 file of a vault's shape: the signatures and version,
/// a header that ends at once, 64 bytes where its hash and HMAC are
/// (`salt` tells two vaults apart), and a stream of one block (`fill`
/// tells two copies of a vault apart) and the empty block.
fn vault(salt: u8, fill: u8) -> Vec<u8> {
    let mut v = faraday_codec::vault_file::SIGNATURE.to_vec();
    v.extend_from_slice(&[0, 0, 4, 0]);
    v.extend_from_slice(&[0, 0, 0, 0, 0]);
    v.extend_from_slice(&[salt; 64]);
    v.extend_from_slice(&[9; 32]);
    v.extend_from_slice(&65_536u32.to_le_bytes());
    v.resize(v.len() + 65_536, fill);
    v.extend_from_slice(&[9; 32]);
    v.extend_from_slice(&0u32.to_le_bytes());
    v
}

#[test]
fn a_vault_goes_back_over_its_own_file() {
    let dir = stick("over");
    fs::write(dir.join("vault.kdbx"), vault(1, 0)).unwrap();
    fs::write(dir.join("vault-2.kdbx"), vault(2, 0)).unwrap();
    let id = dir.to_str().unwrap();
    // The second vault, sealed and offered as vault.kdbx, still replaces
    // vault-2.kdbx: the name is not the vault's identity.
    let c = crate::compute();
    let seen = [faraday_crypto::sha256(&vault(2, 0))];
    let wrote = faraday_storage::write_vault(&c, id, "vault.kdbx", &vault(2, 9), &seen).unwrap();
    assert_eq!(wrote, "vault-2.kdbx");
    assert_eq!(fs::read(dir.join("vault-2.kdbx")).unwrap(), vault(2, 9));
    assert_eq!(fs::read(dir.join("vault.kdbx")).unwrap(), vault(1, 0));
    // A vault no file on the stick has is written as a new one.
    let wrote = faraday_storage::write_vault(&c, id, "vault.kdbx", &vault(3, 0), &[]).unwrap();
    assert_eq!(wrote, "vault-3.kdbx");
}

#[test]
fn an_interrupted_write_back_is_finished() {
    let dir = stick("interrupted");
    // The original was deleted and the copy never renamed.
    fs::write(dir.join(".faraday-vault.kdbx.part"), vault(1, 5)).unwrap();
    // A copy cut short beside its original.
    fs::write(dir.join("vault-2.kdbx"), vault(2, 0)).unwrap();
    fs::write(dir.join(".faraday-vault-2.kdbx.part"), &vault(2, 7)[..1000]).unwrap();
    faraday_storage::finish_vault_writes(&dir);
    assert_eq!(fs::read(dir.join("vault.kdbx")).unwrap(), vault(1, 5));
    assert_eq!(fs::read(dir.join("vault-2.kdbx")).unwrap(), vault(2, 0));
    assert!(!dir.join(".faraday-vault.kdbx.part").exists());
    assert!(!dir.join(".faraday-vault-2.kdbx.part").exists());
}

/// A vault written over its same vault on a stick keeps the name the
/// stick has for it, whatever the app calls it, unless the person renamed
/// it For the stick: then it takes the new name (`docs/FLOWS.md`
/// decision 61 item 2). One copy is left either way.
#[test]
fn a_vault_keeps_its_name_on_the_stick_unless_renamed_for_the_stick() {
    use faraday_storage::Sticks;
    let dir = stick("renamed");
    fs::write(dir.join("savings.kdbx"), vault(1, 0)).unwrap();
    let id = dir.to_str().unwrap();
    let mut sticks = faraday_storage::Dirs::default();
    let c = crate::compute();
    let seen = [faraday_crypto::sha256(&vault(1, 0))];
    let wrote = sticks
        .write(&c, id, "vault.kdbx", &vault(1, 9), &seen, true, false)
        .unwrap();
    assert_eq!(wrote, "savings.kdbx", "a rename made on the stick stays");
    assert_eq!(fs::read(dir.join("savings.kdbx")).unwrap(), vault(1, 9));
    assert!(!dir.join("vault.kdbx").exists());
    let seen = [faraday_crypto::sha256(&vault(1, 9))];
    let wrote = sticks
        .write(&c, id, "mine.kdbx", &vault(1, 5), &seen, true, true)
        .unwrap();
    assert_eq!(wrote, "mine.kdbx");
    assert_eq!(fs::read(dir.join("mine.kdbx")).unwrap(), vault(1, 5));
    assert!(!dir.join("savings.kdbx").exists());
}

/// A file on a stick renamed in place keeps its bytes; a name another
/// file has, ignoring case, is refused with nothing written, and said to
/// be the same vault where it is (`docs/FLOWS.md` decision 61 item 4).
#[test]
fn a_rename_on_a_stick_keeps_the_bytes_and_refuses_a_name_taken() {
    use faraday_files::Unrenamed;
    use faraday_storage::Sticks;
    let dir = stick("rename");
    fs::write(dir.join("notes.txt"), b"the notes").unwrap();
    fs::write(dir.join("Other.txt"), b"other").unwrap();
    fs::write(dir.join("vault.kdbx"), vault(1, 0)).unwrap();
    fs::write(dir.join("vault-copy.kdbx"), vault(1, 0)).unwrap();
    let id = dir.to_str().unwrap();
    let mut sticks = faraday_storage::Dirs::default();
    let c = crate::compute();
    let known = vec![vec![faraday_crypto::sha256(&vault(1, 0))]];
    assert_eq!(
        sticks.rename(&c, id, "notes.txt", "other.txt", &known),
        Err(Unrenamed::Taken)
    );
    assert_eq!(
        sticks.rename(&c, id, "vault-copy.kdbx", "VAULT.kdbx", &known),
        Err(Unrenamed::SameVault)
    );
    assert_eq!(fs::read(dir.join("notes.txt")).unwrap(), b"the notes");
    assert!(dir.join("vault-copy.kdbx").exists());
    assert_eq!(
        sticks.rename(&c, id, "notes.txt", "march.txt", &known),
        Ok(())
    );
    assert_eq!(fs::read(dir.join("march.txt")).unwrap(), b"the notes");
    assert!(!dir.join("notes.txt").exists());
}
