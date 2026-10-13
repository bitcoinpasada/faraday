//! The Companion refuses what the device keeps to itself, and nothing it
//! hands out: every file the test kit and `tools/fixtures/` hold, and
//! every code in their pictures, that the device reads as a secret is one
//! `faraday_intake::secret` refuses, and every one it reads as something
//! a wallet or a coordinator uses is let in.

use faraday_core::testkit;
use faraday_core::wallet::{FileKind, classify};

/// The test kit's files, the files in `tools/fixtures/`, and what each
/// picture's codes hold, by name.
fn everything() -> Vec<(String, Vec<u8>)> {
    let mut out = testkit::files().expect("test kit");
    out.extend(testkit::backup_files().expect("backup test stick"));
    let root = format!("{}/../../tools/fixtures", env!("CARGO_MANIFEST_DIR"));
    let mut dirs = vec![std::path::PathBuf::from(root)];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).expect("fixtures") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                let name = path.display().to_string();
                out.push((name, std::fs::read(&path).expect("fixture")));
            }
        }
    }
    let codes: Vec<(String, Vec<u8>)> = out
        .iter()
        .filter(|(name, _)| name.ends_with(".png"))
        .flat_map(|(name, bytes)| {
            faraday_files::qr_in_png(bytes)
                .unwrap_or_default()
                .into_iter()
                .map(move |code| (format!("{name} (code)"), code))
        })
        .collect();
    assert!(!codes.is_empty());
    out.extend(codes);
    out
}

#[test]
fn the_companion_refuses_what_the_device_keeps_and_nothing_else() {
    let mut secrets = 0;
    let mut public = 0;
    for (name, bytes) in everything() {
        let kind = classify(&name, &bytes);
        let refused = faraday_intake::secret(&bytes);
        match kind {
            FileKind::Words
            | FileKind::SeedPart
            | FileKind::Vault
            | FileKind::Backup
            | FileKind::Kdbx
            | FileKind::Carry
            | FileKind::Entries => {
                assert!(refused.is_some(), "{name} ({kind:?}) is let in");
                secrets += 1;
            }
            FileKind::Psbt
            | FileKind::Wallet
            | FileKind::Key
            | FileKind::Share
            | FileKind::Message
            | FileKind::Transaction => {
                assert_eq!(refused, None, "{name} ({kind:?}) is refused");
                public += 1;
            }
            _ => {}
        }
    }
    assert!(
        secrets > 4 && public > 20,
        "{secrets} secrets, {public} public"
    );
}
