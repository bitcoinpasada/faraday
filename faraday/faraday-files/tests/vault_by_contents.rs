//! A vault on a stick is told by its contents, whatever its name: the
//! KDBX 4 signature and a vault's length (`docs/VAULT-HARDENING.md` §2.1).
//! A copy made by a tool that writes short names only is still a vault,
//! and no write goes over it as over any other file.

use std::collections::BTreeMap;

use faraday_codec::vault_file::SIGNATURE;
use faraday_crypto::work::Compute;
use faraday_files::{
    OnTaken, Place, Unrenamed, WriteRefused, Zeroed, finish_vault_writes, listing, rename,
    vault_names, write_any, write_replacing, write_vault, write_vault_over,
};

#[derive(Default)]
struct Memory {
    files: BTreeMap<String, Vec<u8>>,
    /// A stick that keeps a hidden part file cut short, so its read-back
    /// differs.
    cuts_parts: bool,
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
        let kept = if self.cuts_parts && name.starts_with(".faraday-") {
            &bytes[..bytes.len() - 1]
        } else {
            bytes
        };
        self.files.insert(name.into(), kept.to_vec());
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

/// A whole KDBX 4 file of a vault's shape: the signatures and version,
/// a header that ends at once, 64 bytes where its hash and HMAC are
/// (`salt` tells two vaults apart) and a stream of one block and the
/// empty block.
fn kdbx4(salt: u8, body: usize) -> Vec<u8> {
    let mut v = SIGNATURE.to_vec();
    v.extend_from_slice(&[0, 0, 4, 0]);
    v.extend_from_slice(&[0, 0, 0, 0, 0]);
    v.extend_from_slice(&[salt; 64]);
    v.extend_from_slice(&[9; 32]);
    v.extend_from_slice(&(body as u32).to_le_bytes());
    v.resize(v.len() + body, 5);
    v.extend_from_slice(&[9; 32]);
    v.extend_from_slice(&0u32.to_le_bytes());
    v
}

/// A vault of the smallest size.
fn vault(salt: u8) -> Vec<u8> {
    kdbx4(salt, 65_536)
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
    place.files.insert("VAULT~1.KDB".into(), vault(1));
    place.files.insert("vault".into(), vault(2));
    place.files.insert("named.kdbx".into(), vault(3));
    // Named `.kdbx` and holding other bytes: not a vault.
    place.files.insert("fake.kdbx".into(), vec![7; 65_800]);
    // Other bytes of a vault's length, and the signature at another length.
    place.files.insert("data.bin".into(), vec![7; 65_800]);
    let mut short = SIGNATURE.to_vec();
    short.extend_from_slice(&[0, 0, 4, 0]);
    short.extend_from_slice(&[0; 100]);
    place.files.insert("short.bin".into(), short);
    let files = place.files().unwrap();
    assert_eq!(
        vault_names(&compute(), &mut place, &files),
        ["VAULT~1.KDB", "named.kdbx", "vault"]
    );
}

/// The first vault format is no longer read: a `.fvlt` file, at the length
/// that format had, is no vault; it is listed as any other file.
#[test]
fn a_fvlt_file_is_listed_as_a_plain_file() {
    let mut place = Memory::default();
    let mut old = b"FVLT".to_vec();
    old.resize(53 + 4 * (65_536 + 40), 0);
    place.files.insert("old.fvlt".into(), old);
    let files = place.files().unwrap();
    assert!(vault_names(&compute(), &mut place, &files).is_empty());
    assert!(listing(&mut place).iter().any(|(n, _)| n == "old.fvlt"));
}

#[test]
fn a_vault_whose_stream_does_not_end_with_the_file_is_not_written_back() {
    let mut place = Memory::default();
    let mut odd = vault(1);
    odd[113..117].copy_from_slice(&70_000u32.to_le_bytes());
    assert!(write_vault(&compute(), &mut place, "v.kdbx", &odd, &[]).is_err());
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
    place.files.insert(part("mine.kdbx"), vault(1));
    // A crafted part file for a name no vault is written under.
    place.files.insert(part("autorun.inf"), vault(2));
    place.files.insert(part("a:b.kdbx"), vault(3));
    // The short name of a vault that was replaced: kept as a `.kdbx`.
    place.files.insert(part("VAULT~1.KDB"), vault(4));
    finish_vault_writes(&mut place);
    assert_eq!(place.files["mine.kdbx"], vault(1));
    assert!(!place.files.contains_key("autorun.inf"));
    assert!(!place.files.contains_key("a:b.kdbx"));
    assert_eq!(place.files["VAULT~1.KDB.kdbx"], vault(4));
}

#[test]
fn a_replacing_write_never_goes_over_a_vault_under_another_name() {
    let mut place = Memory::default();
    place.files.insert("VAULT~1.KDB".into(), vault(1));
    let wrote = write_replacing(&mut place, "VAULT~1.KDB", b"notes").unwrap();
    assert_ne!(wrote, "VAULT~1.KDB");
    assert_eq!(place.files["VAULT~1.KDB"], vault(1));
}

#[test]
fn a_replacing_write_never_removes_or_overwrites_a_vault() {
    // Another vault of the same name: the old one stays, the new one goes
    // beside it.
    let mut place = Memory::default();
    place.files.insert("vault.kdbx".into(), vault(1));
    let wrote = write_any(&compute(), &mut place, "vault.kdbx", &vault(2), true).unwrap();
    assert_eq!(wrote, "vault-2.kdbx");
    assert_eq!(place.files["vault.kdbx"], vault(1));
    // Whatever name's case or contents the write carries, a vault on the
    // stick stays, and the new file goes beside it.
    let old = b"KDBX vault bytes".to_vec();
    for (name, bytes) in [
        ("VAULT.KDBX", b"KDBX other".as_slice()),
        ("vault.kdbx", b"not a vault".as_slice()),
    ] {
        let mut place = Memory::default();
        place.files.insert("vault.kdbx".into(), old.clone());
        let wrote = write_any(&compute(), &mut place, name, bytes, true).unwrap();
        assert_ne!(wrote.to_ascii_lowercase(), "vault.kdbx", "{name}");
        assert_eq!(place.files["vault.kdbx"], old, "{name}");
    }
}

#[test]
fn a_sealed_vault_goes_back_over_its_own_copy_under_a_short_name() {
    let mut place = Memory::default();
    place.files.insert("VAULT~1.KDB".into(), vault(1));
    let mut sealed = vault(1);
    sealed[60] = 9;
    let wrote = write_vault(
        &compute(),
        &mut place,
        "vault.kdbx",
        &sealed,
        &[sha(&vault(1))],
    )
    .unwrap();
    assert_eq!(wrote, "VAULT~1.KDB");
    assert_eq!(place.files["VAULT~1.KDB"], sealed);
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
    place.files.insert("old.kdbx".into(), sealed_again(1, 1));
    place.files.insert("other.kdbx".into(), sealed_again(1, 7));
    let new = sealed_again(1, 3);
    let seen = [sha(&sealed_again(1, 1)), sha(&sealed_again(1, 2))];
    let wrote = write_vault(&compute(), &mut place, "vault.kdbx", &new, &seen).unwrap();
    assert_eq!(wrote, "old.kdbx");
    assert_eq!(place.files["old.kdbx"], new);
    assert_eq!(place.files["other.kdbx"], sealed_again(1, 7));
    assert_eq!(place.files.len(), 2);
}

#[test]
fn a_vault_no_copy_of_which_is_on_the_stick_is_written_as_a_new_file() {
    let mut place = Memory::default();
    place.files.insert("vault.kdbx".into(), sealed_again(1, 7));
    let new = sealed_again(1, 3);
    let wrote = write_vault(
        &compute(),
        &mut place,
        "vault.kdbx",
        &new,
        &[sha(&sealed_again(1, 1))],
    )
    .unwrap();
    assert_eq!(wrote, "vault-2.kdbx");
    assert_eq!(place.files["vault.kdbx"], sealed_again(1, 7));
    assert_eq!(place.files["vault-2.kdbx"], new);
}

#[test]
fn two_copies_on_the_stick_are_refused_naming_both() {
    let mut place = Memory::default();
    place.files.insert("a.kdbx".into(), sealed_again(1, 1));
    place.files.insert("b.kdbx".into(), sealed_again(1, 2));
    let seen = [sha(&sealed_again(1, 1)), sha(&sealed_again(1, 2))];
    let why = write_vault(
        &compute(),
        &mut place,
        "vault.kdbx",
        &sealed_again(1, 3),
        &seen,
    )
    .unwrap_err();
    assert!(why.contains("a.kdbx") && why.contains("b.kdbx"), "{why}");
    assert_eq!(place.files["a.kdbx"], sealed_again(1, 1));
    assert_eq!(place.files["b.kdbx"], sealed_again(1, 2));
    assert_eq!(place.files.len(), 2);
}

#[test]
fn a_write_back_a_pulled_stick_cut_short_leaves_the_copy_or_the_new_file_whole() {
    // Pulled after the new file was written and the original removed.
    let mut place = Memory::default();
    place.files.insert(part("vault.kdbx"), sealed_again(1, 3));
    finish_vault_writes(&mut place);
    assert_eq!(place.files["vault.kdbx"], sealed_again(1, 3));
    // Pulled before: the original stays and the part file goes.
    let mut place = Memory::default();
    place.files.insert("vault.kdbx".into(), sealed_again(1, 1));
    place.files.insert(part("vault.kdbx"), sealed_again(1, 3));
    finish_vault_writes(&mut place);
    assert_eq!(place.files["vault.kdbx"], sealed_again(1, 1));
    assert_eq!(place.files.len(), 1);
}

#[test]
fn a_rename_onto_a_name_is_the_same_vault_only_for_copies_the_session_knows() {
    let mut place = Memory::default();
    place.files.insert("a.kdbx".into(), sealed_again(1, 1));
    place.files.insert("b.kdbx".into(), sealed_again(1, 2));
    place.files.insert("c.kdbx".into(), sealed_again(2, 2));
    let one = vec![sha(&sealed_again(1, 1)), sha(&sealed_again(1, 2))];
    let c = compute();
    // Both are copies of one vault the session knows.
    assert_eq!(
        rename(
            &c,
            &mut place,
            "a.kdbx",
            "B.kdbx",
            std::slice::from_ref(&one)
        ),
        Err(Unrenamed::SameVault)
    );
    // The same header, but the session knows no copies: the name is taken.
    assert_eq!(
        rename(&c, &mut place, "a.kdbx", "B.kdbx", &[]),
        Err(Unrenamed::Taken)
    );
    // One is a copy and the other is not: taken.
    assert_eq!(
        rename(&c, &mut place, "a.kdbx", "c.kdbx", &[one]),
        Err(Unrenamed::Taken)
    );
    assert_eq!(place.files.len(), 3);
}

#[test]
fn bytes_that_begin_with_the_signature_but_are_no_vault_are_written_as_a_file() {
    let mut place = Memory::default();
    let mut bytes = SIGNATURE.to_vec();
    bytes.extend_from_slice(&[0, 0, 4, 0]);
    bytes.extend_from_slice(b"just text");
    write_any(&compute(), &mut place, "x.kdbx", &bytes, false).unwrap();
    assert_eq!(place.files["x.kdbx"], bytes);
}

/// A file of exactly `len` bytes that begins as a KDBX 4 file does.
fn of_len(len: usize) -> Vec<u8> {
    let mut v = SIGNATURE.to_vec();
    v.extend_from_slice(&[0, 0, 4, 0]);
    v.resize(len, 7);
    v
}

#[test]
fn the_shortest_and_longest_vault_are_told_and_one_byte_either_side_is_not() {
    // The header with its hash and HMAC and the stream's empty block; the
    // longest is the largest plaintext (128 MiB) stored as a program may
    // have left it, too long to build here: its length is asked of the
    // rule.
    let least = 349;
    let mut place = Memory::default();
    place.files.insert("Passwords.kdbx".into(), of_len(least));
    place.files.insert("short.kdbx".into(), of_len(least - 1));
    let files = place.files().unwrap();
    assert_eq!(
        vault_names(&compute(), &mut place, &files),
        ["Passwords.kdbx"]
    );
    let most = faraday_files::MAX_VAULT_READ;
    assert!(faraday_codec::vault_file::is_vault_len(most));
    assert!(!faraday_codec::vault_file::is_vault_len(most + 1));
}

/// A write of `new` as `vault.kdbx`, no copy of it proved, `on_taken` said.
fn taken_write(place: &mut Memory, new: &[u8], on_taken: OnTaken) -> Result<String, WriteRefused> {
    write_vault_over(
        &compute(),
        place,
        "vault.kdbx",
        new,
        &[sha(&sealed_again(1, 1))],
        true,
        false,
        on_taken,
    )
}

fn stick_with_stale() -> Memory {
    let mut place = Memory::default();
    place.files.insert("Vault.kdbx".into(), sealed_again(1, 7));
    place
}

#[test]
fn asking_about_a_same_name_file_changes_nothing_and_gives_its_sha() {
    let mut place = stick_with_stale();
    let refused = taken_write(&mut place, &sealed_again(1, 3), OnTaken::Ask).unwrap_err();
    assert_eq!(
        refused,
        WriteRefused::NameTaken {
            on_stick: "Vault.kdbx".into(),
            sha: Some(sha(&sealed_again(1, 7)))
        },
        "the name matches ignoring case"
    );
    assert_eq!(place.files.len(), 1);
    assert_eq!(place.files["Vault.kdbx"], sealed_again(1, 7));
}

#[test]
fn asking_with_no_file_of_the_name_writes_it() {
    let mut place = Memory::default();
    place.files.insert("other.kdbx".into(), sealed_again(1, 7));
    let new = sealed_again(1, 3);
    assert_eq!(
        taken_write(&mut place, &new, OnTaken::Ask).unwrap(),
        "vault.kdbx"
    );
    assert_eq!(place.files["vault.kdbx"], new);
}

#[test]
fn a_proved_copy_is_replaced_under_asking_with_no_question() {
    let mut place = Memory::default();
    place.files.insert("vault.kdbx".into(), sealed_again(1, 1));
    let new = sealed_again(1, 3);
    assert_eq!(
        taken_write(&mut place, &new, OnTaken::Ask).unwrap(),
        "vault.kdbx"
    );
    assert_eq!(place.files["vault.kdbx"], new);
    assert_eq!(place.files.len(), 1);
}

#[test]
fn going_over_a_file_with_the_sha_shown_replaces_it() {
    let mut place = stick_with_stale();
    let new = sealed_again(1, 3);
    let wrote = taken_write(&mut place, &new, OnTaken::Over(sha(&sealed_again(1, 7)))).unwrap();
    assert_eq!(wrote, "Vault.kdbx");
    assert_eq!(place.files.len(), 1);
    assert_eq!(place.files["Vault.kdbx"], new);
}

#[test]
fn going_over_a_file_that_changed_since_it_was_shown_is_refused() {
    let mut place = stick_with_stale();
    let refused = taken_write(
        &mut place,
        &sealed_again(1, 3),
        OnTaken::Over(sha(b"other")),
    );
    assert!(
        matches!(refused, Err(WriteRefused::Failed(_))),
        "{refused:?}"
    );
    assert_eq!(place.files.len(), 1);
    assert_eq!(place.files["Vault.kdbx"], sealed_again(1, 7));
}

#[test]
fn going_beside_leaves_the_old_file_and_takes_a_free_name() {
    let mut place = stick_with_stale();
    let new = sealed_again(1, 3);
    let wrote = taken_write(&mut place, &new, OnTaken::Beside).unwrap();
    assert_eq!(wrote, "vault-2.kdbx");
    assert_eq!(place.files["Vault.kdbx"], sealed_again(1, 7));
    assert_eq!(place.files["vault-2.kdbx"], new);
}

#[test]
fn a_failed_read_back_going_over_leaves_the_old_file() {
    let mut place = stick_with_stale();
    place.cuts_parts = true;
    let over = OnTaken::Over(sha(&sealed_again(1, 7)));
    assert!(taken_write(&mut place, &sealed_again(1, 3), over).is_err());
    assert_eq!(place.files["Vault.kdbx"], sealed_again(1, 7));
    assert_eq!(place.files.len(), 1, "no part file is left");
}
