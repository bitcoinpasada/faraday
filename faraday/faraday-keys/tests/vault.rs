//! A vault opened and sealed again, and the keys it holds loaded: a key
//! record of the "abandon" words loads as the fingerprint BIP-39 and
//! BIP-32 publish for them, a vault sealed after a change opens with the
//! change, and a wrong passphrase opens nothing.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_keys::Keys;
use faraday_keys::vault::{self, OpenVaults, gpg_field, is_secret};
use faraday_keys::{Part, Reveal};
use faraday_vault::records::{field, kind};
use faraday_vault::{Contents, Cost, Record};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// A token for the tests' key work, counting nothing (`docs/COMPUTE.md`
/// §3). Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

/// A vault of the smallest slot holding one key record of the "abandon"
/// words under `passphrase`. Costs the cheapest Argon2id `create` allows.
fn made(c: &Compute, passphrase: &str) -> Vec<u8> {
    let m = Mnemonic::parse(c, Language::English, ABANDON).expect("the words");
    let contents = Contents {
        records: vec![
            Record::new(kind::KEY)
                .with(field::KEY, &faraday_vault::records::words_payload(&m))
                .with(field::KEY_LABEL, b"Test"),
        ],
    };
    let cost = Cost {
        memory_kib: faraday_vault::CREATE_MIN_MEMORY_KIB,
        passes: faraday_vault::CREATE_MIN_PASSES,
        lanes: 1,
    };
    vault::create(
        c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[passphrase],
        &[contents],
        &[7u8; 32],
    )
    .expect("a vault")
}

#[test]
fn a_key_in_a_vault_loads_as_the_key_the_bips_publish() {
    let c = compute();
    let file = made(&c, "first passphrase");
    let mut store = OpenVaults::default();
    assert!(
        store.unlock(&c, file.clone(), "another one").is_err(),
        "a wrong passphrase opens nothing"
    );
    let (id, _) = store.unlock(&c, file, "first passphrase").expect("opens");
    let mut keys = Keys::on(Network::Mainnet);
    let all = (0..store.records(id).len()).collect();
    let loaded = store.load_from_vault(&c, &mut keys, id, &all);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].0, [0x73, 0xc5, 0xda, 0x0a]);
    assert_eq!(keys.len(), 1);
}

#[test]
fn a_vault_sealed_after_a_change_opens_with_it() {
    let c = compute();
    let file = made(&c, "first passphrase");
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, "first passphrase").expect("opens");
    store.insert(id, Record::new(kind::NOTE).with(field::NOTE, b"kept"));
    let sealed = store.sealed(id, &[9u8; 24], None).expect("seals");
    store.close(id);
    let (again, _) = store
        .unlock(&c, sealed, "first passphrase")
        .expect("the sealed vault opens");
    assert_eq!(store.records(again).of(kind::NOTE).count(), 1);
    assert_eq!(store.records(again).of(kind::KEY).count(), 1);
}

/// A vault of the smallest slot holding `records`, under `passphrase`.
fn made_with(c: &Compute, passphrase: &str, records: Vec<Record>) -> Vec<u8> {
    let cost = Cost {
        memory_kib: faraday_vault::CREATE_MIN_MEMORY_KIB,
        passes: faraday_vault::CREATE_MIN_PASSES,
        lanes: 1,
    };
    vault::create(
        c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[passphrase],
        &[Contents { records }],
        &[7u8; 32],
    )
    .expect("a vault")
}

fn entry() -> Record {
    Record::new(kind::ENTRY)
        .with(field::TITLE, b"Mail")
        .with(field::USERNAME, b"me")
        .with(field::PASSWORD, b"hunter2")
        .with(field::NOTES, b"recovery code 1234")
}

#[test]
fn a_record_shows_its_public_fields_and_only_the_length_of_a_secret_one() {
    let c = compute();
    let file = made_with(&c, "pass", vec![entry()]);
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    let r = store.records(id).get(0).expect("the entry");
    assert_eq!(r.public_text(field::TITLE), Some("Mail"));
    assert_eq!(r.public_text(field::USERNAME), Some("me"));
    assert_eq!(r.public(field::PASSWORD), None);
    assert!(r.has(field::PASSWORD));
    assert_eq!(r.len(field::PASSWORD), Some(7));
    assert_eq!(r.public_all(field::NOTES).count(), 0);
    assert!(!r.has(field::TOTP));
    assert!(is_secret(kind::ENTRY, field::PASSWORD));
    assert!(!is_secret(kind::ENTRY, field::TITLE));
}

#[test]
fn a_secret_leaves_a_record_only_through_a_reveal_that_names_it() {
    let c = compute();
    let file = made_with(&c, "pass", vec![entry()]);
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    let shown = |why| store.reveal(&c, id, 0, field::PASSWORD, why).to_string();
    assert_eq!(shown(Reveal::VaultEntry(Part::Password)), "hunter2");
    assert_eq!(
        shown(Reveal::VaultEntry(Part::EntryNotes)),
        "",
        "another part is not the password"
    );
    assert_eq!(
        &*store.reveal(
            &c,
            id,
            0,
            field::NOTES,
            Reveal::VaultEntry(Part::EntryNotes)
        ),
        "recovery code 1234"
    );
    assert_eq!(
        &*store.reveal(&c, id, 0, field::TITLE, Reveal::VaultEntry(Part::Password)),
        "",
        "a public field is read as public"
    );
}

/// A GPG key made at 1,780,000,000 with no revocation kept yet, as a key
/// made before revocations were kept.
fn gpg_key() -> Record {
    let at = 1_780_000_000u32.to_le_bytes();
    Record::new(kind::GPG)
        .with(gpg_field::PRIMARY, &[1; 32])
        .with(gpg_field::CREATED, &at)
        .with(gpg_field::SUBKEY, &[2; 32])
        .with(gpg_field::SUBKEY_CREATED, &at)
        .with(gpg_field::USER_ID, b"Faraday test")
}

/// Whoever has a key's revocation certificate can revoke it (F32): the
/// kept one is read only as its date, and leaves only as a `.rev` file
/// asked for by name.
#[test]
fn a_kept_revocation_leaves_only_as_a_rev_file_asked_for_by_name() {
    let c = compute();
    let file = made_with(&c, "pass", vec![gpg_key()]);
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    let rev = Reveal::OutboxFile(Part::Revocation);
    assert!(
        store.gpg_revocation_file(id, 0, rev).is_none(),
        "none kept yet"
    );
    assert!(store.gpg_keep_revocation(id, 0, 1_780_000_100));
    assert!(
        !store.gpg_keep_revocation(id, 0, 1_780_000_200),
        "one is kept"
    );
    let r = store.records(id).get(0).expect("the key");
    assert!(r.has(gpg_field::REVOCATION));
    assert_eq!(r.public(gpg_field::REVOCATION), None);
    assert_eq!(vault::gpg_revocation_kept_at(r), Some(1_780_000_100));
    assert!(
        store
            .gpg_revocation_file(id, 0, Reveal::OutboxFile(Part::Words))
            .is_none(),
        "another part is not the revocation"
    );
    let kept = store.gpg_revocation_file(id, 0, rev).expect("the kept one");
    assert!(kept.starts_with("This is a revocation certificate for the OpenPGP key:"));
    let new = store
        .gpg_new_revocation_file(
            id,
            0,
            1_780_000_300,
            faraday_pgp::Reason::Superseded,
            "",
            rev,
        )
        .expect("a new one");
    assert_ne!(**new, **kept);
    assert_eq!(
        vault::gpg_revocation_kept_at(store.records(id).get(0).expect("the key")),
        Some(1_780_000_100),
        "a new one does not replace the kept one"
    );
}

#[test]
fn a_secret_field_is_written_as_secret_and_a_public_one_as_public() {
    let c = compute();
    let file = made_with(&c, "pass", vec![entry()]);
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    assert!(!store.set_public(id, 0, field::PASSWORD, b"x"));
    assert!(!store.set_secret(id, 0, field::TITLE, &zeroize::Zeroizing::new(b"x".to_vec())));
    assert!(store.set_public(id, 0, field::TITLE, b"Work"));
    assert!(store.set_secret(
        id,
        0,
        field::PASSWORD,
        &zeroize::Zeroizing::new(b"correct horse".to_vec())
    ));
    assert_eq!(
        store.records(id).get(0).unwrap().public_text(field::TITLE),
        Some("Work")
    );
    assert_eq!(
        &*store.reveal(
            &c,
            id,
            0,
            field::PASSWORD,
            Reveal::VaultEntry(Part::Password)
        ),
        "correct horse"
    );
}

#[test]
fn every_edit_moves_the_version_and_a_read_does_not() {
    let c = compute();
    let file = made_with(&c, "pass", vec![entry()]);
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    let v0 = store.version(id);
    let _ = store.records(id).iter().count();
    let _ = store.reveal(
        &c,
        id,
        0,
        field::PASSWORD,
        Reveal::VaultEntry(Part::Password),
    );
    assert_eq!(store.version(id), v0, "reading changes nothing");
    store.set_public(id, 0, field::TITLE, b"Work");
    let v1 = store.version(id);
    assert!(v1 > v0);
    store.insert(id, Record::new(kind::NOTE).with(field::NOTE, b"n"));
    let v2 = store.version(id);
    assert!(v2 > v1);
    store.remove(id, 1);
    assert!(store.version(id) > v2);
}

#[test]
fn what_is_read_of_the_words_follows_the_record_it_was_read_from() {
    let c = compute();
    let m12 = Mnemonic::parse(&c, Language::English, ABANDON).expect("12 words");
    let zoo24 = format!("{} vote", ["zoo"; 23].join(" "));
    let m24 = Mnemonic::parse(&c, Language::English, &zoo24).expect("24 words");
    let key = |m: &Mnemonic| {
        Record::new(kind::KEY).with(field::KEY, &faraday_vault::records::words_payload(m))
    };
    let file = made_with(&c, "pass", vec![key(&m12)]);
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    assert_eq!(store.word_count(&c, id, 0), Some(12));
    let before = store.words_id(&c, id, 0).copied();
    store.replace(id, 0, key(&m24));
    assert_eq!(
        store.word_count(&c, id, 0),
        Some(24),
        "the new record's words"
    );
    assert_ne!(store.words_id(&c, id, 0).copied(), before);
}

#[test]
fn a_key_record_copied_between_vaults_is_the_same_key_there() {
    let c = compute();
    let from = made(&c, "first");
    let to = made_with(&c, "second", Vec::new());
    let mut store = OpenVaults::default();
    let (a, _) = store.unlock(&c, from, "first").expect("opens");
    let (b, _) = store.unlock(&c, to, "second").expect("opens");
    assert_eq!(store.records(b).len(), 0);
    store.copy_record(a, 0, b, None).expect("copied");
    assert_eq!(store.records(b).len(), 1);
    let mut keys = Keys::on(Network::Mainnet);
    let loaded = store.load_from_vault(&c, &mut keys, b, &[0].into_iter().collect());
    assert_eq!(loaded[0].0, [0x73, 0xc5, 0xda, 0x0a]);
    let bare = store
        .copy_out(a, 0, Some(field::KEY_LABEL))
        .expect("a copy");
    assert!(
        !bare.view().has(field::KEY_LABEL),
        "lacking the field asked"
    );
}

// ---- A GPG key restored from its paperkey (docs/GPG-PAPERKEY.md §8) ----

const MADE: u32 = 1_780_000_000;
const YEAR: u32 = 31_536_000;

/// A key as Make makes one, with seeds that start with zero bytes (a
/// shorter secret MPI).
fn gpg_made() -> faraday_keys::gpg::Made {
    let mut primary = zeroize::Zeroizing::new([0x31u8; 32]);
    primary[0] = 0;
    let mut subkey = zeroize::Zeroizing::new([0x42u8; 32]);
    subkey[..2].fill(0);
    faraday_keys::gpg::create(
        &primary,
        &subkey,
        MADE,
        "Test Person <test@example.com>".to_string(),
        2 * YEAR,
    )
}

/// The certificate armoured, read for a restore.
fn read_cert(packets: &[u8]) -> faraday_pgp::Certificate {
    let text = faraday_pgp::armor(faraday_pgp::Armor::PublicKey, packets);
    faraday_pgp::Certificate::read(text.as_bytes()).expect("a certificate of Faraday's shape")
}

/// Two vaults opened from one empty file, Make's key in the first: the
/// store, their ids, and the key's paperkey as its page shows it.
fn with_made(c: &Compute, made: &faraday_keys::gpg::Made) -> (OpenVaults, [vault::VaultId; 2]) {
    let file = made_with(c, "pass", Vec::new());
    let mut store = OpenVaults::default();
    let (a, _) = store.unlock(c, file.clone(), "pass").expect("opens");
    let (b, _) = store.unlock(c, file, "pass").expect("opens");
    store.insert(a, &made.key).expect("kept");
    (store, [a, b])
}

/// The lines typed back, one at a time, as printed: number, colon, bytes
/// and CRC.
fn typed_back(restore: &mut faraday_keys::gpg::Restore, paperkey: &str) {
    for (i, l) in paperkey.lines().enumerate() {
        assert_eq!(restore.next(), i as u32 + 1);
        restore.line(i as u32 + 1, l).expect("each line is taken");
    }
    assert!(restore.complete());
}

/// Test 16: a key made by Make, its paperkey revealed and typed back
/// against its certificate: the record restored is Make's, field for
/// field and byte for byte, the kept revocation included.
#[test]
fn a_key_restored_from_its_paperkey_is_the_record_make_wrote() {
    let c = compute();
    let made = gpg_made();
    let (mut store, [a, b]) = with_made(&c, &made);
    let shown = store
        .gpg_paperkey(store.records(a).get(0).unwrap(), Reveal::GpgPaperkey)
        .expect("the paperkey");
    let mut restore = faraday_keys::gpg::Restore::new(read_cert(&made.certificate));
    assert_eq!(restore.lines(), None, "the count waits for the lengths");
    typed_back(&mut restore, &shown);
    assert_eq!(restore.lines(), Some(7));
    assert_eq!(
        restore.fingerprints(),
        &[made.facts.fingerprint, made.facts.subkey_fingerprint]
    );
    let restored = restore.finish(&c).expect("restored");
    assert_eq!(restored.set_aside, 0);
    assert_eq!(restored.revoked, None);
    assert_eq!(restored.made.certificate, made.certificate);
    assert_eq!(restored.made.facts.fingerprint, made.facts.fingerprint);
    assert_eq!(restored.made.facts.expiry, made.facts.expiry);
    store.insert(b, &restored.made.key).expect("kept");
    let (mine, theirs) = (
        store.records(a).get(0).unwrap(),
        store.records(b).get(0).unwrap(),
    );
    for n in 1..=9 {
        assert_eq!(mine.has(n), theirs.has(n), "field {n}");
        assert_eq!(mine.public(n), theirs.public(n), "field {n}");
    }
    let rev = Reveal::OutboxFile(Part::Revocation);
    assert_eq!(
        **store.gpg_revocation_file(a, 0, rev).unwrap(),
        **store.gpg_revocation_file(b, 0, rev).unwrap(),
        "the kept revocation"
    );
    // The two vaults, sealed under one nonce, are the same bytes: every
    // field of the two records, secret ones included, is the same.
    let nonce = [3u8; 24];
    assert_eq!(store.sealed(a, &nonce, None), store.sealed(b, &nonce, None));
}

/// A line mistyped is refused and nothing of it kept; once typed right,
/// the restore goes on; a line typed again clears it and those after.
#[test]
fn a_mistyped_line_is_refused_and_typed_again() {
    let c = compute();
    let made = gpg_made();
    let (store, [a, _]) = with_made(&c, &made);
    let shown = store
        .gpg_paperkey(store.records(a).get(0).unwrap(), Reveal::GpgPaperkey)
        .expect("the paperkey");
    let lines: Vec<&str> = shown.lines().collect();
    let mut restore = faraday_keys::gpg::Restore::new(read_cert(&made.certificate));
    restore.line(1, lines[0]).expect("line 1");
    let wrong = lines[1].replacen(": 00 ", ": 01 ", 1);
    assert_eq!(
        restore.line(2, &wrong),
        Err(faraday_keys::gpg::LineRefused::Line(
            faraday_pgp::paperkey::LineError::Crc
        ))
    );
    assert_eq!(restore.next(), 2, "nothing of it is kept");
    assert_eq!(
        restore.line(3, lines[2]),
        Err(faraday_keys::gpg::LineRefused::Line(
            faraday_pgp::paperkey::LineError::Number { typed: 3 }
        ))
    );
    assert_eq!(
        restore.line(2, &lines[1].replacen("  2:", "  4:", 1)),
        Err(faraday_keys::gpg::LineRefused::Line(
            faraday_pgp::paperkey::LineError::Number { typed: 4 }
        ))
    );
    for (i, l) in lines.iter().enumerate().skip(1) {
        restore.line(i as u32 + 1, l).expect("taken");
    }
    assert!(restore.complete());
    restore.again(3);
    assert!(!restore.complete());
    assert_eq!(restore.next(), 3);
    for (i, l) in lines.iter().enumerate().skip(2) {
        restore.line(i as u32 + 1, l).expect("taken again");
    }
    assert!(restore.finish(&c).is_ok());
    // A lock: every line forgotten, the certificate kept.
    restore.wipe();
    assert_eq!(restore.next(), 1);
    assert_eq!(restore.checked(), 0);
    assert_eq!(restore.certificate().fingerprint(), made.facts.fingerprint);
    assert!(restore.finish(&c).is_err(), "nothing to restore from");
}

/// `gpg --verify` of `sig` over `data` with `cert` imported, at `now`.
/// Fails when gpg is missing.
fn gnupg_verifies(cert: &[u8], data: &[u8], sig: &str, now: u32) -> Option<String> {
    let dir = std::env::temp_dir().join(format!("faraday-keys-restore-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let gpg = |args: &[&str]| {
        std::process::Command::new("gpg")
            .arg("--homedir")
            .arg(&dir)
            .args(["--batch", "--no-tty", "--status-fd", "1"])
            .arg("--faked-system-time")
            .arg(format!("{now}!"))
            .args(args)
            .output()
    };
    let files = [
        ("cert.asc", cert),
        ("data.txt", data),
        ("data.txt.asc", sig.as_bytes()),
    ];
    for (name, bytes) in files {
        std::fs::write(dir.join(name), bytes).unwrap();
    }
    let path = |n: &str| dir.join(n).to_str().unwrap().to_string();
    let out = match gpg(&["--import", &path("cert.asc")]) {
        Ok(o) => o,
        Err(e) => panic!("gpg is not installed ({e}); see docs/DEVELOPING.md"),
    };
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = gpg(&["--verify", &path("data.txt.asc"), &path("data.txt")]).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Tests 17 and 18: the restored key signs a file with the same bytes as
/// Make's at the same time, which `gpg --verify` takes; Renew and Make a
/// revocation on it give what they give on Make's.
#[test]
fn a_restored_key_signs_renews_and_revokes_as_makes_does() {
    let c = compute();
    let made = gpg_made();
    let (mut store, [a, b]) = with_made(&c, &made);
    let shown = store
        .gpg_paperkey(store.records(a).get(0).unwrap(), Reveal::GpgPaperkey)
        .expect("the paperkey");
    let mut restore = faraday_keys::gpg::Restore::new(read_cert(&made.certificate));
    typed_back(&mut restore, &shown);
    let restored = restore.finish(&c).expect("restored");
    store.insert(b, &restored.made.key).expect("kept");

    let data = b"A file signed after a restore.\n";
    let at = MADE + 600;
    let mine = store.gpg_sign_armored(a, 0, data, at).expect("signs");
    let theirs = store.gpg_sign_armored(b, 0, data, at).expect("signs");
    assert_eq!(mine, theirs);
    let cert = faraday_pgp::armor(faraday_pgp::Armor::PublicKey, &restored.made.certificate);
    if let Some(said) = gnupg_verifies(cert.as_bytes(), data, &theirs, at + 60) {
        assert!(
            said.contains("GOODSIG") && said.contains("VALIDSIG"),
            "{said}"
        );
    }

    let renewed = MADE + YEAR;
    let expiry = Some(renewed - MADE + 2 * YEAR);
    assert_eq!(
        store.gpg_certificate(a, 0, renewed, expiry),
        store.gpg_certificate(b, 0, renewed, expiry)
    );
    let rev = Reveal::OutboxFile(Part::Revocation);
    let (mine, theirs) = (
        store
            .gpg_new_revocation_file(a, 0, renewed, faraday_pgp::Reason::Retired, "Gone", rev)
            .expect("a revocation"),
        store
            .gpg_new_revocation_file(b, 0, renewed, faraday_pgp::Reason::Retired, "Gone", rev)
            .expect("a revocation"),
    );
    assert_eq!(**mine, **theirs);
}

/// A certificate with the key's revocation in it restores marked revoked
/// at the revocation's time, as Make's record is once marked.
#[test]
fn a_revoked_certificate_restores_a_record_marked_revoked() {
    let c = compute();
    let made = gpg_made();
    let (store, [a, _]) = with_made(&c, &made);
    let shown = store
        .gpg_paperkey(store.records(a).get(0).unwrap(), Reveal::GpgPaperkey)
        .expect("the paperkey");
    let when = MADE + 900;
    let rev = store
        .gpg_new_revocation_file(
            a,
            0,
            when,
            faraday_pgp::Reason::Compromised,
            "",
            Reveal::OutboxFile(Part::Revocation),
        )
        .expect("a revocation");
    // The revocation's packets, from the armour after the file's colon.
    let armoured = rev.split_once("\n:").expect("the colon").1;
    // Where OpenPGP puts it: straight after the primary key's packet.
    let cert = &made.certificate;
    let primary_end = 2 + usize::from(cert[1]);
    let mut both = cert[..primary_end].to_vec();
    both.extend(packets_of(armoured));
    both.extend_from_slice(&cert[primary_end..]);
    let mut restore = faraday_keys::gpg::Restore::new(read_cert(&both));
    typed_back(&mut restore, &shown);
    let restored = restore.finish(&c).expect("restored");
    assert_eq!(restored.revoked, Some(when));
    assert_eq!(
        restored.made.key.view().public(gpg_field::REVOKED),
        Some(&when.to_le_bytes()[..])
    );
}

/// The packets of one armoured block.
fn packets_of(armoured: &str) -> Vec<u8> {
    let body: String = armoured
        .lines()
        .skip_while(|l| !l.is_empty())
        .skip(1)
        .take_while(|l| !l.starts_with('='))
        .collect();
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for ch in body.bytes() {
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        };
        acc = acc << 6 | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

/// paperkey 1.6's output for the key of [`paper_made`]: its base16 lines,
/// comment header and all, and its raw form
/// (`tools/vectors/paperkey/README.md`).
const PRINTED: &str = include_str!("../../../tools/vectors/paperkey/faraday-key.txt");
const RAW: &[u8] = include_bytes!("../../../tools/vectors/paperkey/faraday-key.raw");

/// The key paperkey printed [`PRINTED`] and [`RAW`] from, as Make makes it
/// (`faraday-pgp`'s example `paperkey_inputs`).
fn paper_made() -> faraday_keys::gpg::Made {
    let mut primary = zeroize::Zeroizing::new([0x11u8; 32]);
    primary[0] = 0;
    let mut subkey = zeroize::Zeroizing::new([0x22u8; 32]);
    subkey[..2].fill(0);
    faraday_keys::gpg::create(
        &primary,
        &subkey,
        MADE,
        "Test Person <test@example.com>".to_string(),
        2 * YEAR,
    )
}

/// The base16 lines of [`PRINTED`], its comments left out.
fn printed_lines() -> Vec<&'static str> {
    PRINTED
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect()
}

/// A paperkey read from a code: paperkey 1.6's base16 lines, comments and
/// all, and its raw form each fill every line, checked, and restore the
/// record that typing the lines restores.
#[test]
fn a_paperkey_code_in_either_of_paperkeys_forms_restores_the_key() {
    let c = compute();
    let made = paper_made();
    let typed = {
        let mut r = faraday_keys::gpg::Restore::new(read_cert(&made.certificate));
        for (i, l) in printed_lines().into_iter().enumerate() {
            r.line(i as u32 + 1, l).expect("each line is taken");
        }
        r.finish(&c).expect("restored from the lines")
    };
    for code in [PRINTED.as_bytes(), RAW] {
        let mut r = faraday_keys::gpg::Restore::new(read_cert(&made.certificate));
        r.paperkey_code(code).expect("the code is taken");
        assert!(r.complete());
        assert_eq!(r.checked(), 7, "every line, the CRC line included");
        assert_eq!(
            r.fingerprints(),
            &[made.facts.fingerprint, made.facts.subkey_fingerprint]
        );
        let restored = r.finish(&c).expect("restored from the code");
        assert_eq!(restored.made.certificate, made.certificate);
        assert_eq!(restored.made.certificate, typed.made.certificate);
        assert_eq!(restored.made.facts.fingerprint, made.facts.fingerprint);
    }
}

/// A paperkey code that does not hold is refused where typing it would
/// be, and the lines typed before it are left as they were.
#[test]
fn a_paperkey_code_that_does_not_hold_is_refused_and_keeps_the_lines_typed() {
    use faraday_keys::gpg::{CodeRefused, LineRefused};
    use faraday_pgp::Refusal;
    use faraday_pgp::paperkey::LineError;
    let made = paper_made();
    let mut r = faraday_keys::gpg::Restore::new(read_cert(&made.certificate));
    r.line(1, printed_lines()[0]).expect("line 1");
    // A digit changed on line 3: refused at line 3, as paperkey refuses it.
    let wrong = PRINTED.replacen("  3: 11", "  3: 12", 1);
    assert_eq!(
        r.paperkey_code(wrong.as_bytes()),
        Err(CodeRefused::Line(3, LineRefused::Line(LineError::Crc)))
    );
    // A byte of the raw form changed: its CRC does not match.
    let mut raw = RAW.to_vec();
    raw[30] ^= 1;
    assert_eq!(
        r.paperkey_code(&raw),
        Err(CodeRefused::Paperkey(Refusal::Crc))
    );
    // The lines without the whole data's CRC.
    let cut: String = printed_lines()[..6]
        .iter()
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(r.paperkey_code(cut.as_bytes()), Err(CodeRefused::Short));
    // Something after the whole data's CRC.
    let more = format!("{PRINTED}  8: 00 000000\n");
    assert_eq!(r.paperkey_code(more.as_bytes()), Err(CodeRefused::After(8)));
    // Something else entirely.
    assert_eq!(
        r.paperkey_code(b"cHNidP8BAH0CAAAAAQ"),
        Err(CodeRefused::NotPaperkey)
    );
    // Another key's paperkey: refused at line 1.
    let other = gpg_made();
    let mut theirs = faraday_keys::gpg::Restore::new(read_cert(&other.certificate));
    assert!(matches!(
        theirs.paperkey_code(PRINTED.as_bytes()),
        Err(CodeRefused::Line(
            1,
            LineRefused::Paperkey(Refusal::OtherKey(_))
        ))
    ));
    assert!(matches!(
        theirs.paperkey_code(RAW),
        Err(CodeRefused::Paperkey(Refusal::OtherKey(_)))
    ));
    // Line 1 typed before is still there.
    assert_eq!(r.checked(), 1);
    assert_eq!(r.next(), 2);
}
