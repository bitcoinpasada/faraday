//! What the Companion never takes (`docs/COMPANION.md` §5, §11): every
//! kind of secret that stays on the Faraday device, by camera and by
//! paste, shows its one line and leaves the store as it was. The samples
//! are published vectors where there are some, and Faraday's own files.

mod common;

use common::*;
use faraday_companion::route::Refusal;
use faraday_companion::store::{self, Kept, Kind};
use faraday_companion::{Act, Companion, CompanionEvent, Page};
use faraday_intake::Secret;
use faraday_strings::EN;

/// The line a refusal shows.
fn line(r: Refusal) -> &'static str {
    match r {
        Refusal::Secret(s) => secret_line(s),
        Refusal::RecoveryKey => EN.companion_refuse_recovery_key,
    }
}

/// The line a refusal of `s` shows.
fn secret_line(s: Secret) -> &'static str {
    match s {
        Secret::Bip39Words => EN.companion_refuse_words,
        Secret::UrBip39 => EN.companion_refuse_ur_bip39,
        Secret::SeedQr => EN.companion_refuse_seedqr,
        Secret::CompactSeedQr => EN.companion_refuse_compact_seedqr,
        Secret::Codex32 => EN.companion_refuse_codex32,
        Secret::Slip39Share => EN.companion_refuse_slip39,
        Secret::ExtendedPrivateKey => EN.companion_refuse_xprv,
        Secret::Wif => EN.companion_refuse_wif,
        Secret::UrSeed => EN.companion_refuse_ur_seed,
        Secret::UrPrivateKey => EN.companion_refuse_ur_private_key,
        Secret::EncryptedBackup => EN.companion_refuse_backup,
        Secret::KeePass => EN.companion_refuse_keepass,
        Secret::Vault => EN.companion_refuse_vault,
        Secret::ThresholdCarry => EN.companion_refuse_carry,
        Secret::Totp => EN.companion_refuse_totp,
        Secret::TotpMigration => EN.companion_refuse_totp_migration,
        Secret::VaultEntries => EN.companion_refuse_entries,
    }
}

const WORDS: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// SeedSigner's SeedQR specification's 12-word test seed.
const SEEDQR: &str = "073318950739065415961602009907670428187212261116";
const COMPACT: &[u8] = b"[\xbd\x9dq\xa8\xecy\x90\x83\x1a\xff5\x9dBeE";

fn master() -> osk_bip::bitcoin::bip32::Xpriv {
    use std::str::FromStr;
    let x = json("tools/vectors/bip39/vectors.json")["english"][0][3]
        .as_str()
        .expect("xprv")
        .to_string();
    osk_bip::bitcoin::bip32::Xpriv::from_str(&x).expect("BIP-39's vector")
}

fn cbor_map(pairs: &[(u8, Vec<u8>)]) -> Vec<u8> {
    let mut out = faraday_qr::cbor::head(5, pairs.len() as u64);
    for (k, v) in pairs {
        out.extend(faraday_qr::cbor::head(0, u64::from(*k)));
        out.extend_from_slice(v);
    }
    out
}

fn cbor_text(s: &str) -> Vec<u8> {
    let mut out = faraday_qr::cbor::head(3, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
    out
}

/// One sample of every kind.
fn samples() -> Vec<(Secret, Vec<u8>)> {
    let m = master();
    let xprv = m.to_string();
    let wif = osk_bip::bitcoin::PrivateKey::from_slice(
        &m.private_key.secret_bytes(),
        osk_bip::bitcoin::NetworkKind::Main,
    )
    .expect("a key")
    .to_wif();
    let codex32 = json("tools/vectors/bip93/vectors.json")["secrets"][0]["string"]
        .as_str()
        .expect("codex32")
        .to_string();
    let slip39 = json("tools/vectors/slip39/vectors.json")
        .as_array()
        .expect("vectors")
        .iter()
        .find(|c| c[2].as_str() != Some(""))
        .and_then(|c| c[1][0].as_str().map(str::to_string))
        .expect("a share");
    let seed = cbor_map(&[(
        1,
        faraday_qr::cbor::bytes(&[
            0xc7, 0x09, 0x85, 0x80, 0x12, 0x5e, 0x2a, 0xb0, 0x98, 0x12, 0x53, 0x46, 0x8b, 0x2d,
            0xbc, 0x52,
        ]),
    )]);
    let mut list = faraday_qr::cbor::head(4, 12);
    for w in WORDS.split(' ') {
        list.extend(cbor_text(w));
    }
    let bip39 = cbor_map(&[(1, list), (2, cbor_text("en"))]);
    let mut key = vec![0u8];
    key.extend(m.private_key.secret_bytes());
    let hdkey = cbor_map(&[
        (1, vec![0xF5]),
        (3, faraday_qr::cbor::bytes(&key)),
        (4, faraday_qr::cbor::bytes(m.chain_code.as_bytes())),
    ]);
    vec![
        (Secret::Bip39Words, WORDS.as_bytes().to_vec()),
        (Secret::SeedQr, SEEDQR.as_bytes().to_vec()),
        (Secret::CompactSeedQr, COMPACT.to_vec()),
        (Secret::Codex32, codex32.into_bytes()),
        (Secret::Slip39Share, slip39.into_bytes()),
        (Secret::ExtendedPrivateKey, xprv.into_bytes()),
        (Secret::Wif, wif.into_bytes()),
        (
            Secret::UrSeed,
            faraday_qr::ur::single("crypto-seed", &seed).into_bytes(),
        ),
        (
            Secret::UrBip39,
            faraday_qr::ur::single("crypto-bip39", &bip39).into_bytes(),
        ),
        (
            Secret::UrPrivateKey,
            faraday_qr::ur::single("crypto-hdkey", &hdkey).into_bytes(),
        ),
        (
            Secret::EncryptedBackup,
            repo("tools/vectors/backup/backup-v3-words-12-english.oskb"),
        ),
        (
            Secret::KeePass,
            repo("tools/vectors/backup/backup-kdbx-words-12-english.kdbx"),
        ),
        // A vault is larger than any code; the camera sees its head.
        (
            Secret::Vault,
            repo("tools/vectors/vault/vault-one.ofv")[..2000].to_vec(),
        ),
        (
            Secret::ThresholdCarry,
            repo("tools/vectors/psbt/wallet-threshold-carry.osk"),
        ),
        (
            Secret::Totp,
            b"otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example".to_vec(),
        ),
        (
            Secret::TotpMigration,
            b"otpauth-migration://offline?data=CjEKCkhlbGxvId6tvu8SGEV4YW1wbGU6YWxpY2VAZ29vZ2xlLmNvbRoHRXhhbXBsZTAC".to_vec(),
        ),
        (
            Secret::VaultEntries,
            b"title: Bank\nusername: alice\npassword: correct horse battery staple\n".to_vec(),
        ),
    ]
}

/// A phone holding one file already, on Scan a Faraday QR code.
fn holding() -> Companion {
    let mut c = phone();
    let kept = vec![Kept::new("spend.psbt".into(), Kind::Psbt, 1, psbt())];
    c.companion_event(CompanionEvent::Received {
        bytes: store::write(&kept),
    });
    c.press(Act::Receive);
    drain(&mut c);
    c
}

/// `c` refused `kind` with its line and kept nothing.
fn refused(c: &mut Companion, kind: Refusal, before: &[u8], how: &str) {
    assert_eq!(c.page, Page::Refused(kind), "{kind:?} by {how}");
    assert!(shows(c, line(kind)), "{kind:?} by {how}: {:?}", c.texts());
    let (_, ours) = drain(c);
    assert_eq!(
        kept_store(&ours),
        None,
        "{kind:?} by {how}: the store was written"
    );
    assert_eq!(
        kept_kept(&ours),
        None,
        "{kind:?} by {how}: the kept store was written"
    );
    assert!(
        !asked_unlock(&ours),
        "{kind:?} by {how}: the unlock was asked"
    );
    assert_eq!(c.store_bytes(), before, "{kind:?} by {how}");
    assert_eq!(c.received().len(), 1);
}

#[test]
fn every_secret_by_camera_is_refused_with_its_line_and_nothing_kept() {
    for (kind, bytes) in samples() {
        let mut c = holding();
        let before = c.store_bytes();
        show(&mut c, &code_of(&bytes));
        refused(&mut c, kind.into(), &before, "camera");
    }
}

#[test]
fn every_secret_by_paste_is_refused_with_its_line_and_nothing_kept() {
    for (kind, bytes) in samples() {
        let mut c = holding();
        let before = c.store_bytes();
        // A clipboard holds text: a file that is not text is pasted as
        // the Faraday file envelope that carries it.
        let text = match std::str::from_utf8(&bytes) {
            Ok(t) if !bytes.contains(&0) => t.to_string(),
            _ => String::from_utf8(
                faraday_qr::envelope::pack("secret.bin", &bytes, "file").expect("an envelope"),
            )
            .expect("text"),
        };
        paste(&mut c, &text);
        refused(&mut c, kind.into(), &before, "paste");
    }
}

#[test]
fn a_secret_sent_in_parts_is_refused_once_whole_and_the_parts_go_with_it() {
    let mut c = holding();
    let before = c.store_bytes();
    let parts = faraday_qr::bbqr::encode('U', WORDS.as_bytes(), 30).expect("parts");
    assert!(parts.len() > 2);
    for p in &parts {
        if c.page != Page::Receive {
            break;
        }
        show(&mut c, &code_of(p.as_bytes()));
    }
    refused(&mut c, Secret::Bip39Words.into(), &before, "BBQr parts");
    // Scanning on starts afresh: the parts read before are gone.
    c.press(Act::ScanAgain);
    show(&mut c, &code_of(parts[0].as_bytes()));
    let n = parts.len();
    assert!(
        c.texts()
            .iter()
            .any(|t| t.starts_with(&format!("1 of {n}"))),
        "{:?}",
        c.texts()
    );
}

#[test]
fn every_secret_shared_in_is_refused_whole_and_both_stores_stay_as_they_were() {
    for (kind, bytes) in samples() {
        // From anywhere: Received open, and Wallet descriptions open.
        let mut c = holding();
        let before = c.store_bytes();
        c.companion_event(CompanionEvent::SharedIn {
            name: Some("shared.txt".into()),
            bytes: bytes.clone(),
        });
        refused(&mut c, kind.into(), &before, "share");
        let mut c = holding();
        c.press(Act::Back);
        c.press(Act::Descriptions);
        unlock(&mut c, &[]);
        assert!(c.kept().is_some_and(|k| k.is_empty()));
        c.companion_event(CompanionEvent::SharedIn { name: None, bytes });
        refused(
            &mut c,
            kind.into(),
            &before,
            "share, Wallet descriptions open",
        );
        assert!(c.kept().is_some_and(|k| k.is_empty()));
    }
}

/// What a BitLocker recovery key looks like when it arrives: alone, and
/// under the line the 0.1.2 Starter's code carried.
fn recovery_keys() -> Vec<String> {
    vec![RECOVERY_KEY.to_string(), starter_key_text()]
}

#[test]
fn a_recovery_key_by_camera_paste_or_share_is_refused_and_nothing_is_kept() {
    for text in recovery_keys() {
        let mut c = holding();
        let before = c.store_bytes();
        show(&mut c, &code_of(text.as_bytes()));
        refused(&mut c, Refusal::RecoveryKey, &before, "camera");

        let mut c = holding();
        paste(&mut c, &text);
        refused(&mut c, Refusal::RecoveryKey, &before, "paste");

        // Shared in from anywhere, Wallet descriptions open as well.
        let mut c = holding();
        c.companion_event(CompanionEvent::SharedIn {
            name: Some("key.txt".into()),
            bytes: text.clone().into_bytes(),
        });
        refused(&mut c, Refusal::RecoveryKey, &before, "share");
        let mut c = holding();
        c.press(Act::Back);
        c.press(Act::Descriptions);
        unlock(&mut c, &[]);
        c.companion_event(CompanionEvent::SharedIn {
            name: None,
            bytes: text.into_bytes(),
        });
        refused(
            &mut c,
            Refusal::RecoveryKey,
            &before,
            "share, descriptions open",
        );
        assert!(c.kept().is_some_and(|k| k.is_empty()));
    }
}

#[test]
fn a_recovery_key_sent_in_parts_is_refused_once_whole() {
    let mut c = holding();
    let before = c.store_bytes();
    let parts = faraday_qr::bbqr::encode('U', RECOVERY_KEY.as_bytes(), 30).expect("parts");
    for p in &parts {
        if c.page != Page::Receive {
            break;
        }
        show(&mut c, &code_of(p.as_bytes()));
    }
    refused(&mut c, Refusal::RecoveryKey, &before, "BBQr parts");
}

#[test]
fn six_digits_in_seven_groups_is_not_a_recovery_key() {
    let mut c = holding();
    paste(&mut c, "205568-492019-108251-336446-650375-210331-584155");
    assert!(!matches!(c.page, Page::Refused(_)), "{:?}", c.page);
}

/// Windows' own "Save to a file" key file.
fn windows_key_file() -> String {
    format!(
        "BitLocker Drive Encryption recovery key\r\n\r\nTo verify that this is the correct recovery key, \
         compare the start of the following identifier with the identifier value displayed on your PC.\r\n\r\n\
         Identifier:\r\n\r\n\t6A0E1F2C-3B4D-4E5F-8A9B-0C1D2E3F4A5B\r\n\r\n\
         If the above identifier matches the one displayed by your PC, then use the following key to unlock your drive.\r\n\r\n\
         Recovery Key:\r\n\r\n{RECOVERY_KEY}\r\n"
    )
}

/// The same file as Windows writes it: UTF-16, little endian, with a mark.
fn utf16_with_mark(text: &str) -> Vec<u8> {
    let mut out = vec![0xFF, 0xFE];
    out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    out
}

#[test]
fn a_windows_key_file_shared_in_is_refused_whole_in_either_encoding() {
    for bytes in [
        utf16_with_mark(&windows_key_file()),
        windows_key_file().into_bytes(),
        // UTF-16 with no mark.
        windows_key_file()
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect(),
    ] {
        let mut c = holding();
        let before = c.store_bytes();
        c.companion_event(CompanionEvent::SharedIn {
            name: Some("BitLocker Recovery Key.txt".into()),
            bytes,
        });
        refused(&mut c, Refusal::RecoveryKey, &before, "key file shared");
    }
}

#[test]
fn a_recovery_key_inside_an_envelope_or_a_key_file_in_parts_is_refused() {
    let file = windows_key_file();
    let env = faraday_qr::envelope::pack("key.txt", file.as_bytes(), "file").expect("env");
    let mut c = holding();
    let before = c.store_bytes();
    show(&mut c, &code_of(&env));
    refused(&mut c, Refusal::RecoveryKey, &before, "envelope");

    let mut c = holding();
    paste(&mut c, &String::from_utf8(env).expect("text"));
    refused(&mut c, Refusal::RecoveryKey, &before, "envelope paste");

    let mut c = holding();
    let parts = faraday_qr::bbqr::encode('U', file.as_bytes(), 60).expect("parts");
    for p in &parts {
        if c.page != Page::Receive {
            break;
        }
        show(&mut c, &code_of(p.as_bytes()));
    }
    refused(&mut c, Refusal::RecoveryKey, &before, "BBQr key file");
}
