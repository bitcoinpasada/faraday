//! A vault on a stick is told by its contents, whatever its name: the
//! magic and one of the vault lengths (`docs/VAULT-HARDENING.md` §2.1).
//! A copy made by a tool that writes short names only is still a vault,
//! and no write goes over it as over any other file.

use std::collections::BTreeMap;

use faraday_codec::vault_file::{MAGIC, file_len};
use faraday_crypto::work::Compute;
use faraday_files::{
    Place, Unrenamed, Zeroed, finish_vault_writes, rename, vault_names, write_any, write_replacing,
    write_vault,
};

#[derive(Default)]
struct Memory {
    files: BTreeMap<String, Vec<u8>>,
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
        self.files.get(name).cloned().ok_or("no such file".into())
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
        Ok(Zeroed {
            next: 2,
            total: 0,
            done: true,
        })
    }
    fn make_empty(&mut self) -> Result<(), String> {
        self.files.clear();
        Ok(())
    }
}

/// A file of a vault's length that begins with the magic; `salt` tells
/// two vaults apart.
fn vault(salt: u8) -> Vec<u8> {
    let mut v = vec![0u8; file_len(65_536)];
    v[..4].copy_from_slice(&MAGIC);
    v[17..21].copy_from_slice(&65_536u32.to_le_bytes());
    v[21..53].fill(salt);
    v
}

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-files' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

#[test]
fn a_vault_is_found_by_its_contents_under_any_name() {
    let mut place = Memory::default();
    place.files.insert("VAULT~1.FVL".into(), vault(1));
    place.files.insert("vault".into(), vault(2));
    place.files.insert("named.fvlt".into(), vault(3));
    // Named `.fvlt` and holding other bytes: not a vault.
    place
        .files
        .insert("fake.fvlt".into(), vec![7; file_len(65_536)]);
    // Other bytes at a vault's length, and the magic at another length.
    place
        .files
        .insert("data.bin".into(), vec![7; file_len(65_536)]);
    let mut short = MAGIC.to_vec();
    short.extend_from_slice(&[0; 100]);
    place.files.insert("short.bin".into(), short);
    let files = place.files().unwrap();
    assert_eq!(
        vault_names(&compute(), &mut place, &files),
        ["VAULT~1.FVL", "named.fvlt", "vault"]
    );
}

#[test]
fn a_vault_whose_header_names_another_slot_length_is_not_written_back() {
    let mut place = Memory::default();
    let mut odd = vault(1);
    odd[17..21].copy_from_slice(&262_144u32.to_le_bytes());
    assert!(write_vault(&compute(), &mut place, "v.fvlt", &odd, &[]).is_err());
    assert!(place.files.is_empty());
}

/// A part file as an interrupted write-back leaves it.
fn part(name: &str) -> String {
    format!(".faraday-{name}.part")
}

#[test]
fn an_interrupted_write_back_finishes_only_under_a_name_a_vault_may_have() {
    let mut place = Memory::default();
    // Its original is gone: the copy takes the name.
    place.files.insert(part("mine.fvlt"), vault(1));
    // A crafted part file for a name no vault is written under.
    place.files.insert(part("autorun.inf"), vault(2));
    place.files.insert(part("a:b.fvlt"), vault(3));
    // The short name of a vault that was replaced: kept as a `.fvlt`.
    place.files.insert(part("VAULT~1.FVL"), vault(4));
    finish_vault_writes(&mut place);
    assert_eq!(place.files["mine.fvlt"], vault(1));
    assert!(!place.files.contains_key("autorun.inf"));
    assert!(!place.files.contains_key("a:b.fvlt"));
    assert_eq!(place.files["VAULT~1.FVL.fvlt"], vault(4));
}

#[test]
fn a_replacing_write_never_goes_over_a_vault_under_another_name() {
    let mut place = Memory::default();
    place.files.insert("VAULT~1.FVL".into(), vault(1));
    let wrote = write_replacing(&mut place, "VAULT~1.FVL", b"notes").unwrap();
    assert_ne!(wrote, "VAULT~1.FVL");
    assert_eq!(place.files["VAULT~1.FVL"], vault(1));
}

#[test]
fn a_replacing_write_never_removes_or_overwrites_a_vault() {
    // Another vault of the same name: the old one stays, the new one goes
    // beside it.
    let mut place = Memory::default();
    place.files.insert("vault.fvlt".into(), vault(1));
    let wrote = write_any(&compute(), &mut place, "vault.fvlt", &vault(2), true).unwrap();
    assert_eq!(wrote, "vault-2.fvlt");
    assert_eq!(place.files["vault.fvlt"], vault(1));
    // Whatever name's case or contents the write carries, a vault on the
    // stick stays, and the new file goes beside it.
    let old = b"FVLT vault bytes".to_vec();
    for (name, bytes) in [
        ("VAULT.FVLT", b"FVLT other".as_slice()),
        ("vault.fvlt", b"not a vault".as_slice()),
    ] {
        let mut place = Memory::default();
        place.files.insert("vault.fvlt".into(), old.clone());
        let wrote = write_any(&compute(), &mut place, name, bytes, true).unwrap();
        assert_ne!(wrote.to_ascii_lowercase(), "vault.fvlt", "{name}");
        assert_eq!(place.files["vault.fvlt"], old, "{name}");
    }
}

#[test]
fn a_sealed_vault_goes_back_over_its_own_copy_under_a_short_name() {
    let mut place = Memory::default();
    place.files.insert("VAULT~1.FVL".into(), vault(1));
    let mut sealed = vault(1);
    sealed[60] = 9;
    let wrote = write_vault(
        &compute(),
        &mut place,
        "vault.fvlt",
        &sealed,
        &[sha(&vault(1))],
    )
    .unwrap();
    assert_eq!(wrote, "VAULT~1.FVL");
    assert_eq!(place.files["VAULT~1.FVL"], sealed);
    assert_eq!(place.files.len(), 1);
}

/// The SHA-256 of a file, as the app keeps it for a copy it has seen.
fn sha(bytes: &[u8]) -> [u8; 32] {
    faraday_crypto::sha256(bytes)
}

/// A vault of the same salt as `vault(salt)`, sealed again: other bytes.
fn sealed_again(salt: u8, mark: u8) -> Vec<u8> {
    let mut v = vault(salt);
    v[60] = mark;
    v
}

#[test]
fn only_the_copy_the_session_has_seen_is_replaced_and_not_a_vault_of_the_same_length() {
    let mut place = Memory::default();
    // An old copy, a later seal of the same vault, and another vault of
    // the same length, which shares the vault's salt by chance.
    place.files.insert("old.fvlt".into(), sealed_again(1, 1));
    place.files.insert("other.fvlt".into(), sealed_again(1, 7));
    let new = sealed_again(1, 3);
    let seen = [sha(&sealed_again(1, 1)), sha(&sealed_again(1, 2))];
    let wrote = write_vault(&compute(), &mut place, "vault.fvlt", &new, &seen).unwrap();
    assert_eq!(wrote, "old.fvlt");
    assert_eq!(place.files["old.fvlt"], new);
    assert_eq!(place.files["other.fvlt"], sealed_again(1, 7));
    assert_eq!(place.files.len(), 2);
}

#[test]
fn a_vault_no_copy_of_which_is_on_the_stick_is_written_as_a_new_file() {
    let mut place = Memory::default();
    place.files.insert("vault.fvlt".into(), sealed_again(1, 7));
    let new = sealed_again(1, 3);
    let wrote = write_vault(
        &compute(),
        &mut place,
        "vault.fvlt",
        &new,
        &[sha(&sealed_again(1, 1))],
    )
    .unwrap();
    assert_eq!(wrote, "vault-2.fvlt");
    assert_eq!(place.files["vault.fvlt"], sealed_again(1, 7));
    assert_eq!(place.files["vault-2.fvlt"], new);
}

#[test]
fn two_copies_on_the_stick_are_refused_naming_both() {
    let mut place = Memory::default();
    place.files.insert("a.fvlt".into(), sealed_again(1, 1));
    place.files.insert("b.fvlt".into(), sealed_again(1, 2));
    let seen = [sha(&sealed_again(1, 1)), sha(&sealed_again(1, 2))];
    let why = write_vault(
        &compute(),
        &mut place,
        "vault.fvlt",
        &sealed_again(1, 3),
        &seen,
    )
    .unwrap_err();
    assert!(why.contains("a.fvlt") && why.contains("b.fvlt"), "{why}");
    assert_eq!(place.files["a.fvlt"], sealed_again(1, 1));
    assert_eq!(place.files["b.fvlt"], sealed_again(1, 2));
    assert_eq!(place.files.len(), 2);
}

#[test]
fn a_write_back_a_pulled_stick_cut_short_leaves_the_copy_or_the_new_file_whole() {
    // Pulled after the new file was written and the original removed.
    let mut place = Memory::default();
    place.files.insert(part("vault.fvlt"), sealed_again(1, 3));
    finish_vault_writes(&mut place);
    assert_eq!(place.files["vault.fvlt"], sealed_again(1, 3));
    // Pulled before: the original stays and the part file goes.
    let mut place = Memory::default();
    place.files.insert("vault.fvlt".into(), sealed_again(1, 1));
    place.files.insert(part("vault.fvlt"), sealed_again(1, 3));
    finish_vault_writes(&mut place);
    assert_eq!(place.files["vault.fvlt"], sealed_again(1, 1));
    assert_eq!(place.files.len(), 1);
}

#[test]
fn a_rename_onto_a_name_is_the_same_vault_only_for_copies_the_session_knows() {
    let mut place = Memory::default();
    place.files.insert("a.fvlt".into(), sealed_again(1, 1));
    place.files.insert("b.fvlt".into(), sealed_again(1, 2));
    place.files.insert("c.fvlt".into(), sealed_again(2, 2));
    let one = vec![sha(&sealed_again(1, 1)), sha(&sealed_again(1, 2))];
    let c = compute();
    // Both are copies of one vault the session knows.
    assert_eq!(
        rename(
            &c,
            &mut place,
            "a.fvlt",
            "B.fvlt",
            std::slice::from_ref(&one)
        ),
        Err(Unrenamed::SameVault)
    );
    // The same header, but the session knows no copies: the name is taken.
    assert_eq!(
        rename(&c, &mut place, "a.fvlt", "B.fvlt", &[]),
        Err(Unrenamed::Taken)
    );
    // One is a copy and the other is not: taken.
    assert_eq!(
        rename(&c, &mut place, "a.fvlt", "c.fvlt", &[one]),
        Err(Unrenamed::Taken)
    );
    assert_eq!(place.files.len(), 3);
}

#[test]
fn bytes_that_begin_with_the_magic_but_are_no_vault_are_written_as_a_file() {
    let mut place = Memory::default();
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(b"just text");
    write_any(&compute(), &mut place, "x.fvlt", &bytes, false).unwrap();
    assert_eq!(place.files["x.fvlt"], bytes);
}
