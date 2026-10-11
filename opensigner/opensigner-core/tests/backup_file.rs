//! The encrypted backup a key is written to and read back from
//! (`docs/PLANNING.md` §16.71 decision 4): Key › Backup › Encrypted
//! backup makes one, and the Load wizard and Scan read one. The format
//! itself, against its published vectors and a second implementation,
//! is `core/osk-backup/tests/vectors.rs`.

mod common;

use std::io::Write;
use std::process::Command as Process;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::backup::BackupStep;
use opensigner_core::load::Step;
use opensigner_core::{ScreenKind, ids, strings};
use osk_backup::DEVICE_PARAMS;
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{Fingerprint, MasterKey, Network};
use osk_shell_api::{Command, Event, FileKind, Key};

/// The 12-word version-2 vector, which this build still opens.
const VECTOR: &str = include_str!("../../../tools/vectors/backup/backup-12-english.hex");

/// The passphrase that vector was written under.
const VECTOR_PASSPHRASE: &str = "correct horse battery staple";

/// The passphrase the flow tests type.
const PASSPHRASE: &str = "a long enough one";

fn unhex(text: &str) -> Vec<u8> {
    let text = text.trim();
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).expect("hex"))
        .collect()
}

/// The 56 bytes the 12-word vector's salt and nonce were taken from.
fn vector_seed() -> [u8; osk_backup::oskb::SEED_LEN] {
    let mut seed = [0u8; osk_backup::oskb::SEED_LEN];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = i as u8;
    }
    seed
}

fn fingerprint_of(m: &Mnemonic, network: Network) -> Fingerprint {
    MasterKey::from_seed(&m.to_seed(b"").expect("a seed"), network).fingerprint()
}

impl Harness {
    /// Types a backup passphrase and presses ✓.
    fn type_passphrase(&mut self, passphrase: &str) {
        self.type_text(passphrase);
        self.key(Key::Enter);
    }

    /// Home › Keys › one key › Backup › Encrypted backup › the form,
    /// which is the step before the passphrase for anything encrypted
    /// (`docs/PLANNING.md` §16.112).
    fn open_encrypted_backup(&mut self, key: usize) {
        self.open_key(key);
        self.tap(ids::DETAIL_BACKUP);
        self.tap(ids::BACKUP_ENCRYPTED);
        self.tap(ids::FORM_CONTINUE);
    }

    /// The bytes "Save to file" handed the shell, and the answer that
    /// the file is there. The name is returned with them.
    fn save_backup(&mut self) -> (String, Vec<u8>) {
        self.seen.clear();
        self.tap(ids::BACKUP_SAVE);
        let written: Vec<(FileKind, String, Vec<u8>)> = self
            .seen
            .iter()
            .filter_map(|c| match c {
                Command::WriteFile {
                    kind,
                    name_hint,
                    bytes,
                } => Some((*kind, name_hint.clone(), bytes.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(written.len(), 1, "one WriteFile: {:?}", self.seen);
        let (kind, name, bytes) = written.into_iter().next().expect("the write");
        assert_eq!(kind, FileKind::Any);
        self.send(Event::FileWritten { kind });
        (name, bytes)
    }

    /// Home › Scan, then `bytes` in front of the camera.
    fn scan_backup(&mut self, bytes: &[u8]) {
        self.go_home();
        self.tap(ids::HOME_SCAN);
        self.scan_bytes(bytes);
    }
}

/// A key backed up and read back is the same key. The passphrase is
/// typed twice to make the backup and once to open it, and nothing in
/// between asks for the words again.
#[test]
fn a_key_backed_up_and_read_back_is_the_same_key() {
    // The file's header is asserted below, so this device stretches at
    // the cost a device stretches at.
    let mut h = Harness::at_device_cost(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    let before = h.app.fingerprints();
    assert_eq!(before.len(), 1);

    h.open_encrypted_backup(0);
    assert_eq!(h.app.backup_step(), Some(BackupStep::Passphrase));
    h.type_passphrase(PASSPHRASE);
    assert_eq!(h.app.backup_step(), Some(BackupStep::PassphraseRepeat));
    h.type_passphrase(PASSPHRASE);
    assert_eq!(h.app.backup_step(), Some(BackupStep::Encrypted));

    let (name, bytes) = h.save_backup();
    assert_eq!(
        osk_backup::oskb::cost_of(&bytes),
        Some(DEVICE_PARAMS),
        "the device writes its own Argon2id costs"
    );
    assert!(
        h.app.texts().iter().any(|t| t.contains(&name)),
        "the row says the saved name: {:?}",
        h.app.texts()
    );

    // A fresh device with nothing on it reads the backup back.
    let mut h = Harness::new(PANEL);
    h.scan_backup(&bytes);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.type_passphrase(PASSPHRASE);
    assert_eq!(h.app.load_step(), Some(Step::Checksum));
    assert!(
        h.app.texts().contains(&String::from("English")),
        "the language comes out of the backup: {:?}",
        h.app.texts()
    );
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    assert_eq!(h.app.fingerprints(), before, "the same key");
}

/// The name a saved backup is offered under carries nothing of the key:
/// a file on a card does not say which wallet it belongs to. Two keys
/// are backed up and both are offered the same name.
#[test]
fn the_saved_name_says_nothing_about_the_key() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    let fingerprint = h.app.fingerprints()[0];

    h.open_encrypted_backup(0);
    h.type_passphrase(PASSPHRASE);
    h.type_passphrase(PASSPHRASE);
    let (name, _) = h.save_backup();
    let hex = format!("{fingerprint}").to_lowercase();
    assert!(
        !name.to_lowercase().contains(&hex),
        "the name carries the fingerprint: {name}"
    );
    assert!(
        name.chars()
            .all(|c| c.is_ascii_lowercase() || c == '-' || c == '.'),
        "no fingerprint and no date: {name}"
    );

    // The Result still says which key it is and what the file is called.
    let texts = h.app.texts();
    assert!(texts.iter().any(|t| t.contains(&name)), "{texts:?}");

    let mut h = Harness::new(PANEL);
    let other = Mnemonic::from_entropy(Language::English, &[9u8; 32]).expect("a key");
    let words: Vec<&str> = other
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    h.start_load(&words);
    h.finish_load(None);
    h.open_encrypted_backup(0);
    h.type_passphrase(PASSPHRASE);
    h.type_passphrase(PASSPHRASE);
    let (other_name, _) = h.save_backup();
    assert_eq!(other_name, name, "every backup is offered the same name");
}

/// A backup whose padding after the entropy is not zero is not a backup
/// this build reads, even under the passphrase that made it: the
/// scanner says so rather than loading a key.
#[test]
fn a_backup_with_a_non_zero_pad_is_refused() {
    let m = Mnemonic::from_entropy(Language::English, &[0u8; 16]).expect("a key");
    let bytes = reseal_with_pad(&m, PASSPHRASE.as_bytes());
    let mut h = Harness::new(PANEL);
    h.scan_backup(&bytes);
    h.type_passphrase(PASSPHRASE);
    assert_eq!(
        h.app.scan_error(),
        Some(strings::EN.scan_reason_backup),
        "the scanner says why"
    );
    assert!(h.app.fingerprints().is_empty(), "nothing was loaded");
}

/// The same, for a 24-word Japanese key: the word count and the
/// wordlist come back out of the backup, not out of a guess.
#[test]
fn a_24_word_japanese_key_comes_back_whole() {
    let m = Mnemonic::from_entropy(Language::Japanese, &[0x42; 32]).expect("a key");
    let seed = vector_seed();
    let bytes =
        osk_backup::oskb::seal(&m, PASSPHRASE.as_bytes(), DEVICE_PARAMS, &seed).expect("a backup");

    let mut h = Harness::new(PANEL);
    let expected = fingerprint_of(&m, h.app.network());
    h.scan_backup(&bytes);
    h.type_passphrase(PASSPHRASE);
    assert_eq!(h.app.load_step(), Some(Step::Checksum));
    let texts = h.app.texts();
    assert!(
        texts.contains(&String::from("Japanese")),
        "the wordlist: {texts:?}"
    );
    assert!(texts.contains(&String::from("24")), "the count: {texts:?}");
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    assert_eq!(h.app.fingerprints(), vec![expected]);
}

/// A passphrase that does not open the backup says so under the field
/// and loads nothing; the backup is still in hand, so the entry stays
/// and the right passphrase still works.
#[test]
fn a_wrong_passphrase_says_so_and_loads_nothing() {
    let mut h = Harness::new(PANEL);
    h.scan_backup(&unhex(VECTOR));
    h.type_passphrase("not the one");
    assert_eq!(h.app.screen(), ScreenKind::Scan, "still on the entry");
    assert!(h.app.fingerprints().is_empty(), "nothing was loaded");
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.backup_wrong_pass)),
        "the field says which: {:?}",
        h.app.texts()
    );

    // Backspace over it and type the right one.
    for _ in 0.."not the one".len() {
        h.key(Key::Backspace);
    }
    h.type_passphrase(VECTOR_PASSPHRASE);
    assert_eq!(h.app.load_step(), Some(Step::Checksum));
}

/// A backup whose header was changed is refused before any passphrase
/// is asked for: the magic says what the bytes are, and the reason says
/// why they cannot be used. Version 1, which this build does not read,
/// is refused the same way.
#[test]
fn a_flipped_header_byte_is_refused_on_the_scanner() {
    let mut bytes = unhex(VECTOR);
    bytes[4] = 1;
    let mut h = Harness::new(PANEL);
    h.scan_backup(&bytes);
    assert_eq!(
        h.app.scan_error(),
        Some(strings::EN.scan_reason_backup),
        "the scanner says why"
    );
    assert!(h.app.fingerprints().is_empty());
}

/// Scan on Home routes a backup the same way the Load wizard's own row
/// does, and the wizard's row opens the scanner for one.
#[test]
fn both_ways_in_reach_the_passphrase_entry() {
    let mut h = Harness::new(PANEL);
    h.open_load();
    h.choose(ids::LOAD_SOURCE_BACKUP, ids::LOAD_SOURCE_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.scan_title_backup)),
        "the title says what is expected: {:?}",
        h.app.texts()
    );
    h.scan_bytes(&unhex(VECTOR));
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.backup_pass_title)),
        "the passphrase entry: {:?}",
        h.app.texts()
    );

    // The same bytes in front of Home's own scanner.
    let mut h = Harness::new(PANEL);
    h.scan_backup(&unhex(VECTOR));
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.backup_pass_title)),
        "the passphrase entry: {:?}",
        h.app.texts()
    );
}

/// A passphrase key's backup holds the words the passphrase is applied
/// to, and the result says so rather than leaving the person to guess.
#[test]
fn a_passphrase_keys_backup_says_it_holds_the_parents_words() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(Some("TREZOR"));
    assert_eq!(h.app.has_passphrase(0), Some(true));

    h.open_encrypted_backup(0);
    h.type_passphrase(PASSPHRASE);
    h.type_passphrase(PASSPHRASE);
    assert_eq!(h.app.backup_step(), Some(BackupStep::Encrypted));
    let texts = h.app.texts();
    assert!(
        texts.contains(&String::from(strings::EN.backup_inside_parent)),
        "the row says what is inside: {texts:?}"
    );
    assert!(!texts.contains(&String::from(strings::EN.backup_inside_words)));
}

/// The backup shows as one QR, not an animated run: the payload is a
/// hundred-odd bytes of ciphertext and fits a single code.
#[test]
fn the_backup_shows_as_one_code() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_encrypted_backup(0);
    h.type_passphrase(PASSPHRASE);
    h.type_passphrase(PASSPHRASE);
    h.tap(ids::BACKUP_SHOW_QR);
    assert_eq!(h.app.backup_step(), Some(BackupStep::EncryptedQr));
    assert_eq!(h.app.qr_visible(), Some(true));
    assert!(
        !h.app
            .texts()
            .contains(&String::from(strings::EN.sign_qr_animated)),
        "no animation to offer: {:?}",
        h.app.texts()
    );
}

/// A file that claims 64 GiB of Argon2id memory is refused on the
/// scanner with the memory it asks for.
#[test]
fn a_file_asking_for_more_memory_than_the_device_has_says_how_much() {
    let m = Mnemonic::from_entropy(Language::English, &[1u8; 32]).expect("a key");
    let mut huge = osk_backup::oskb::seal(&m, PASSPHRASE.as_bytes(), DEVICE_PARAMS, &vector_seed())
        .expect("a backup");
    huge[5..9].copy_from_slice(&(64u32 * 1024 * 1024).to_le_bytes());
    let mut h = Harness::new(PANEL);
    h.scan_backup(&huge);
    let needs = strings::fill1(strings::EN.backup_needs_memory, "65536");
    assert_eq!(
        h.app.scan_error(),
        Some(needs.as_str()),
        "the scanner says how much it needs: {:?}",
        h.app.texts()
    )
}

/// One backup of `m` under `passphrase` whose padding after the entropy
/// is not zero, sealed by hand so its tag verifies: what a file someone
/// edited the format of would look like.
fn reseal_with_pad(m: &Mnemonic, passphrase: &[u8]) -> Vec<u8> {
    use argon2::{Algorithm, Argon2, Block, Params, Version};
    use chacha20poly1305::aead::inout::InOutBuf;
    use chacha20poly1305::{AeadInOut, KeyInit, XChaCha20Poly1305, XNonce};

    let mut bytes =
        osk_backup::oskb::seal(m, passphrase, DEVICE_PARAMS, &vector_seed()).expect("a backup");
    let plain_len = bytes.len() - osk_backup::oskb::HEADER_LEN - 24 - 16;
    let mut plain = vec![0u8; plain_len];
    plain[0] = osk_backup::oskb::KIND_WORDS;
    plain[1..3].copy_from_slice(&34u16.to_le_bytes());
    plain[3] = m.word_count() as u8;
    plain[5..5 + 16].copy_from_slice(m.entropy().expose().as_bytes());
    // The last byte of the words payload, which a 12-word key leaves
    // zero.
    plain[3 + 33] = 1;

    let header = bytes[..osk_backup::oskb::HEADER_LEN].to_vec();
    let salt = &header[17..];
    let params: Params = Params::new(
        DEVICE_PARAMS.memory_kib,
        DEVICE_PARAMS.passes,
        DEVICE_PARAMS.lanes,
        Some(32),
    )
    .expect("the device's costs");
    let mut key = [0u8; 32];
    let mut memory = vec![Block::default(); params.block_count()];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into_with_memory(passphrase, salt, &mut key, &mut memory)
        .expect("Argon2id");
    let at = osk_backup::oskb::HEADER_LEN + 24;
    let mut nonce = [0u8; 24];
    nonce.copy_from_slice(&bytes[osk_backup::oskb::HEADER_LEN..at]);
    let tag = XChaCha20Poly1305::new_from_slice(&key)
        .expect("32-byte key")
        .encrypt_inout_detached(
            &XNonce::from(nonce),
            &header,
            InOutBuf::from(&mut plain[..]),
        )
        .expect("it seals");
    bytes[at..at + plain.len()].copy_from_slice(&plain);
    bytes[at + plain.len()..].copy_from_slice(&tag);
    bytes
}

/// A file in the system temporary directory, removed when it drops.
struct Temp {
    path: std::path::PathBuf,
    file: std::fs::File,
}

impl Write for Temp {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn tempfile() -> Temp {
    let path = std::env::temp_dir().join(format!(
        "osk-backup-vector-{}-{}.oskb",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock")
            .as_nanos()
    ));
    let file = std::fs::File::create(&path).expect("a temporary file");
    Temp { path, file }
}

/// A note sealed on one device and scanned on another asks the
/// passphrase and shows the same text (`docs/PLANNING.md` §16.112 rule
/// 2).
#[test]
fn a_note_read_back_under_its_passphrase_shows_the_same_text() {
    let note = "The third key is with the notary.";
    let bytes = osk_backup::oskb::seal_note(
        note.as_bytes(),
        PASSPHRASE.as_bytes(),
        DEVICE_PARAMS,
        &vector_seed(),
    )
    .expect("a sealed note");

    let mut h = Harness::new(PANEL);
    h.scan_backup(&bytes);
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.backup_pass_title)),
        "a sealed note asks the passphrase: {:?}",
        h.app.texts()
    );
    h.type_passphrase(PASSPHRASE);
    assert_eq!(h.app.screen(), ScreenKind::Note);
    assert!(
        h.app.texts().iter().any(|t| t.contains(note)),
        "the note came back whole: {:?}",
        h.app.texts()
    );
    assert!(
        h.app.fingerprints().is_empty(),
        "a note is not a key and loads none"
    );
}

/// A wallet's recovery sheet, exported sealed and read back, opens as a
/// document and offers to register the wallet its descriptor names.
#[test]
fn a_sheet_round_trips_and_offers_the_wallet() {
    const DESCRIPTOR: &str = "wpkh([73c5da0a/84'/0'/0']xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V/<0;1>/*)";
    let bytes = osk_backup::oskb::seal_sheet(
        DESCRIPTOR.as_bytes(),
        b"Cold storage",
        b"The words are with the notary.",
        PASSPHRASE.as_bytes(),
        DEVICE_PARAMS,
        &vector_seed(),
    )
    .expect("a sealed sheet");

    let mut h = Harness::new(PANEL);
    h.scan_backup(&bytes);
    h.type_passphrase(PASSPHRASE);
    assert_eq!(h.app.screen(), ScreenKind::Note);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t.contains("The words are with")),
        "the sheet's note: {texts:?}"
    );
    assert!(
        texts.contains(&String::from(strings::EN.sheet_add_wallet)),
        "the sheet offers its wallet: {texts:?}"
    );

    h.tap(ids::SHEET_ADD_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    assert_eq!(h.app.wallet_count(), 1, "the wallet was registered");
    assert!(
        h.app.texts().iter().any(|t| t.contains("Cold storage")),
        "the name on the sheet came with it: {:?}",
        h.app.texts()
    );
}

/// What the Key derivation row states for `cost`.
fn kdf_text(cost: osk_backup::Cost) -> String {
    let passes = match cost.passes {
        1 => String::from("1 pass"),
        n => format!("{n} passes"),
    };
    let lanes = match cost.lanes {
        1 => String::from("1 lane"),
        n => format!("{n} lanes"),
    };
    format!("Argon2id \u{00b7} {passes} \u{00b7} {lanes}")
}

/// "Which form?" offers the three forms, each with what opens it, and
/// the plain file only for what holds no key (`docs/PLANNING.md`
/// §16.134 rule 1).
#[test]
fn which_form_lists_the_forms_and_what_opens_each() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_ENCRYPTED);
    assert_eq!(h.app.screen(), ScreenKind::ExportForm);
    let texts = h.app.texts();
    let says = |t: &str| texts.iter().any(|x| x == t);
    for t in [
        "Encrypted backup",
        "OpenSigner \u{00b7} decrypt.py",
        "KDBX 4",
        "a KeePass app",
    ] {
        assert!(says(t), "{t}: {texts:?}");
    }
    assert!(!says("Plain text"), "a key is never plain: {texts:?}");
    assert!(!says("QR"), "a QR is not a form: {texts:?}");

    let mut h = Harness::new(PANEL);
    h.go_home();
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_NOTES);
    h.tap(ids::NOTES_NEW);
    h.type_text("Keys in the safe.");
    h.key(Key::Enter);
    h.tap(ids::NOTE_EXPORT);
    let texts = h.app.texts();
    let says = |t: &str| texts.iter().any(|x| x == t);
    for t in [
        "Encrypted backup",
        "OpenSigner \u{00b7} decrypt.py",
        "KDBX 4",
        "a KeePass app",
        "Plain text",
        "any text editor \u{00b7} not encrypted",
    ] {
        assert!(says(t), "{t}: {texts:?}");
    }
}

/// Every Result after sealing states the format, the cipher, the key
/// derivation the file was made with and what reads it back
/// (`docs/PLANNING.md` §16.134 rule 2).
#[test]
fn a_sealed_result_states_its_cipher_and_key_derivation() {
    let states = |h: &Harness, values: &[&str]| {
        let texts = h.app.texts();
        for label in ["Format", "Cipher", "Key derivation", "Read with"] {
            assert!(texts.iter().any(|t| t == label), "{label}: {texts:?}");
        }
        for value in values {
            assert!(texts.iter().any(|t| t == value), "{value}: {texts:?}");
        }
    };

    // A key's encrypted backup.
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_encrypted_backup(0);
    h.type_passphrase(PASSPHRASE);
    h.type_passphrase(PASSPHRASE);
    let (_, bytes) = h.save_backup();
    let cost = osk_backup::oskb::read_header(&bytes).expect("a header");
    states(
        &h,
        &[
            "OSKB 3",
            "XChaCha20-Poly1305",
            &kdf_text(cost),
            "OpenSigner \u{00b7} decrypt.py",
        ],
    );

    // The same key as a KDBX 4 database.
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_kdbx_backup(0);
    h.type_passphrase(PASSPHRASE);
    h.type_passphrase(PASSPHRASE);
    h.seen.clear();
    h.tap(ids::BACKUP_SAVE);
    let (_, bytes) = h.written();
    let cost = osk_backup::kdbx::cost_of(&bytes).expect("a header");
    states(
        &h,
        &["KDBX 4", "ChaCha20", &kdf_text(cost), "a KeePass app"],
    );

    // A note, sealed.
    let mut h = Harness::new(PANEL);
    h.go_home();
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_NOTES);
    h.tap(ids::NOTES_NEW);
    h.type_text("Keys in the safe.");
    h.key(Key::Enter);
    h.tap(ids::NOTE_EXPORT);
    h.tap(ids::FORM_CONTINUE);
    h.type_passphrase(PASSPHRASE);
    h.type_passphrase(PASSPHRASE);
    assert_eq!(h.app.screen(), ScreenKind::Sealed);
    h.seen.clear();
    h.tap(ids::SEAL_SAVE);
    let (_, bytes) = h.written();
    let cost = osk_backup::oskb::read_header(&bytes).expect("a header");
    states(
        &h,
        &[
            "OSKB 3",
            "XChaCha20-Poly1305",
            &kdf_text(cost),
            "OpenSigner \u{00b7} decrypt.py",
        ],
    );
}

/// "Read an encrypted backup" is a row of Tools › Notes and of a
/// wallet's Recovery sheet, and from either a sealed sheet read from a
/// file opens with "Add this wallet" (`docs/PLANNING.md` §16.134 rule 3).
#[test]
fn a_sealed_sheet_is_read_from_notes_and_from_a_recovery_sheet() {
    const DESCRIPTOR: &str = "wpkh([73c5da0a/84'/0'/0']xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V/<0;1>/*)";
    let s = &strings::EN;
    let sheet = osk_backup::oskb::seal_sheet(
        DESCRIPTOR.as_bytes(),
        b"Cold storage",
        b"The words are with the notary.",
        PASSPHRASE.as_bytes(),
        DEVICE_PARAMS,
        &vector_seed(),
    )
    .expect("a sealed sheet");
    let read_it = |h: &mut Harness| {
        assert_eq!(h.app.screen(), ScreenKind::Scan);
        h.send(Event::CameraUnavailable);
        h.tap(ids::SCAN_FILE);
        h.send(Event::File {
            kind: FileKind::Any,
            bytes: sheet.clone(),
        });
        h.type_passphrase(PASSPHRASE);
        assert!(
            h.app.texts().iter().any(|t| t == s.sheet_add_wallet),
            "the sheet offers its wallet: {:?}",
            h.app.texts()
        );
        h.tap(ids::SHEET_ADD_WALLET);
        assert_eq!(h.app.screen(), ScreenKind::Wallet);
        assert!(
            h.app.texts().iter().any(|t| t.contains("Cold storage")),
            "the wallet the sheet names: {:?}",
            h.app.texts()
        );
    };

    // Tools › Notes.
    let mut h = Harness::new(PANEL);
    h.go_home();
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_NOTES);
    assert!(
        h.app.texts().iter().any(|t| t == s.load_source_backup),
        "the row: {:?}",
        h.app.texts()
    );
    h.tap(ids::NOTES_READ_FILE);
    read_it(&mut h);

    // A wallet's Recovery sheet.
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.add_single_sig(0, 2);
    h.open_wallets();
    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    h.tap(ids::WALLET_SHEET);
    assert!(
        h.app.texts().iter().any(|t| t == s.load_source_backup),
        "the row: {:?}",
        h.app.texts()
    );
    h.tap(ids::SHEET_READ);
    read_it(&mut h);
}

// ---------------------------------------------------------------------------
// KDBX 4, the export a KeePass app opens (`docs/PLANNING.md` §16.112
// pass E2).
// ---------------------------------------------------------------------------

/// The 140 bytes that vector's master seed, IV, salt and inner key were
/// taken from.
fn kdbx_vector_seed() -> [u8; osk_backup::kdbx::SEED_LEN] {
    let mut seed = [0u8; osk_backup::kdbx::SEED_LEN];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = i as u8;
    }
    seed
}

impl Harness {
    /// Home › Keys › one key › Backup › Encrypted backup, with KDBX 4
    /// chosen on "Which form?".
    fn open_kdbx_backup(&mut self, key: usize) {
        self.open_key(key);
        self.tap(ids::DETAIL_BACKUP);
        self.tap(ids::BACKUP_ENCRYPTED);
        self.tap(ids::at(ids::FORM_BASE, 1));
        self.tap(ids::FORM_CONTINUE);
    }

    /// The bytes the last "Save to file" handed the shell, with the
    /// name it was offered under.
    fn written(&mut self) -> (String, Vec<u8>) {
        let written: Vec<(String, Vec<u8>)> = self
            .seen
            .iter()
            .filter_map(|c| match c {
                Command::WriteFile {
                    name_hint, bytes, ..
                } => Some((name_hint.clone(), bytes.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(written.len(), 1, "one WriteFile: {:?}", self.seen);
        written.into_iter().next().expect("the write")
    }
}

/// A key's words, a key's master seed, a note and a wallet's recovery
/// sheet each leave the device as a KDBX 4 database that authenticates
/// under the passphrase typed for it and opens in a KeePass reader.
#[test]
fn every_export_leaves_as_a_kdbx_database() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    let fingerprint = format!("{}", h.app.fingerprints()[0]).to_lowercase();

    // A key's words.
    h.open_kdbx_backup(0);
    h.type_passphrase(PASSPHRASE);
    h.type_passphrase(PASSPHRASE);
    h.seen.clear();
    h.tap(ids::BACKUP_SAVE);
    let (name, bytes) = h.written();
    assert!(
        osk_backup::kdbx::verify(&bytes, PASSPHRASE.as_bytes()),
        "it authenticates"
    );
    assert_eq!(
        name,
        strings::EN.kdbx_file_name,
        "a KDBX file is offered the same name every time, as every backup is"
    );
    let words = ABANDON.join(" ");
    if let Some(printed) = read_kdbx_with_script(&bytes, PASSPHRASE) {
        assert!(printed.contains(&words), "the words: {printed}");
        assert!(printed.contains(&fingerprint), "the title: {printed}");
    }

    // A key with no words: its master seed, as hex.
    let seed_backup = osk_backup::oskb::seal_seed(
        &[0x42u8; 32],
        PASSPHRASE.as_bytes(),
        DEVICE_PARAMS,
        &vector_seed(),
    )
    .expect("a sealed seed");
    let mut h = Harness::new(PANEL);
    h.scan_backup(&seed_backup);
    h.type_passphrase(PASSPHRASE);
    h.add_key();
    assert_eq!(h.app.has_mnemonic(0), Some(false));
    h.open_kdbx_backup(0);
    h.type_passphrase(PASSPHRASE);
    h.type_passphrase(PASSPHRASE);
    h.seen.clear();
    h.tap(ids::BACKUP_SAVE);
    let (_, bytes) = h.written();
    assert!(
        osk_backup::kdbx::verify(&bytes, PASSPHRASE.as_bytes()),
        "it authenticates"
    );
    if let Some(printed) = read_kdbx_with_script(&bytes, PASSPHRASE) {
        assert!(
            printed.contains(&"42".repeat(32)),
            "the seed as hex: {printed}"
        );
    }

    // A note.
    let note = "Where the keys are\nThe second key is with the notary.";
    let mut h = Harness::new(PANEL);
    h.go_home();
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_NOTES);
    h.tap(ids::NOTES_READ_FILE);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: note.as_bytes().to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Note);
    h.tap(ids::NOTE_EXPORT);
    h.tap(ids::at(ids::FORM_BASE, 1));
    h.tap(ids::FORM_CONTINUE);
    h.type_passphrase(PASSPHRASE);
    h.type_passphrase(PASSPHRASE);
    h.seen.clear();
    h.tap(ids::SEAL_SAVE);
    let (name, bytes) = h.written();
    assert!(
        osk_backup::kdbx::verify(&bytes, PASSPHRASE.as_bytes()),
        "it authenticates"
    );
    assert_eq!(name, strings::EN.kdbx_file_name);
    if let Some(printed) = read_kdbx_with_script(&bytes, PASSPHRASE) {
        assert!(printed.contains("title: Where the keys are"), "{printed}");
        assert!(printed.contains("with the notary"), "{printed}");
    }

    // A wallet's recovery sheet.
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.add_single_sig(0, 2);
    h.open_wallets();
    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    h.tap(ids::WALLET_SHEET);
    h.tap(ids::SHEET_EXPORT);
    h.tap(ids::at(ids::FORM_BASE, 1));
    h.tap(ids::FORM_CONTINUE);
    h.type_passphrase(PASSPHRASE);
    h.type_passphrase(PASSPHRASE);
    h.seen.clear();
    h.tap(ids::SEAL_SAVE);
    let (_, bytes) = h.written();
    assert!(
        osk_backup::kdbx::verify(&bytes, PASSPHRASE.as_bytes()),
        "it authenticates"
    );
    if let Some(printed) = read_kdbx_with_script(&bytes, PASSPHRASE) {
        assert!(printed.contains("wpkh("), "the descriptor: {printed}");
    }
}

/// The Argon2id memory the person chose in Settings is what a KDBX
/// file's KDF parameters state, so a KeePass app opening it pays what
/// this device paid.
#[test]
fn a_kdbx_file_states_the_memory_the_person_chose() {
    let cost = osk_backup::Cost {
        memory_kib: opensigner_core::BACKUP_MEMORY[1],
        passes: DEVICE_PARAMS.passes,
        lanes: DEVICE_PARAMS.lanes,
    };
    let bytes = osk_backup::kdbx::write(
        &[osk_backup::kdbx::Item {
            title: b"73c5da0a",
            notes: b"12 words, English",
            secret: b"abandon abandon about",
        }],
        PASSPHRASE.as_bytes(),
        cost,
        &kdbx_vector_seed(),
    )
    .expect("a database");
    assert_eq!(osk_backup::kdbx::cost_of(&bytes), Some(cost));
    assert!(osk_backup::kdbx::verify(&bytes, PASSPHRASE.as_bytes()));
}

/// Runs `tools/backup/read-kdbx.py` over `bytes` and returns what it
/// printed. `None` where the script cannot run here, which is a machine
/// with neither `pykeepass` nor `cryptography`.
fn read_kdbx_with_script(bytes: &[u8], passphrase: &str) -> Option<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("tools/backup/read-kdbx.py");
    let probe = Process::new("python3")
        .arg("-c")
        .arg(
            "import sys, pathlib; sys.path.insert(0, str(pathlib.Path('tools/backup')));\n\
             import decrypt;\n\
             decrypt.argon2id(b'x', bytes(32), 8, 1, 1);\n\
             from cryptography.hazmat.primitives.ciphers import Cipher, algorithms;\n\
             Cipher(algorithms.ChaCha20(bytes(32), bytes(16)), mode=None).encryptor()",
        )
        .current_dir(&root)
        .output()
        .ok()?;
    if !probe.status.success() {
        return None;
    }
    let mut file = tempfile();
    file.write_all(bytes).expect("write the database");
    file.flush().expect("flush");
    let path = file.path.clone();
    let out = Process::new("python3")
        .arg(&script)
        .arg(&path)
        .arg("--passphrase")
        .arg(passphrase)
        .current_dir(&root)
        .output()
        .expect("run the script");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}
