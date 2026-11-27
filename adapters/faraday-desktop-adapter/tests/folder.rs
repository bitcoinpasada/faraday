//! A folder standing for a stick (`FolderPlace`): its answers are the app's words and its reads
//! come in pieces. These tests were `faraday-files`' on `main`, over its `Dir`.

use faraday_desktop_adapter::FolderPlace;
use faraday_files::proto::PIECE;
use faraday_files::{MAX_READ, Place, read_part, read_whole};
use faraday_strings::EN;

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("faraday-pieces-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Bytes that begin as KDBX 4 and are `len` long.
fn vault_of(len: usize) -> Vec<u8> {
    let mut b: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
    b[..8].copy_from_slice(&faraday_codec::vault_file::SIGNATURE);
    b[8..10].copy_from_slice(&0u16.to_le_bytes());
    b[10..12].copy_from_slice(&4u16.to_le_bytes());
    b
}

/// Invariant: A folder stick refuses in the app's own words, never the operating system's message.
/// Regression: A refusal passes the operating system's error text through to the screen.
/// Owner: adapter
///
/// A file that is not there, and a rename onto a name another file has.
#[test]
fn a_folder_refuses_in_the_apps_words() {
    let dir = std::env::temp_dir().join(format!("faraday-files-words-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.psbt"), b"a").unwrap();
    std::fs::write(dir.join("b.psbt"), b"b").unwrap();
    let mut place = FolderPlace(dir.clone());
    assert_eq!(
        place.read("gone.psbt", 1024),
        Err(faraday_text::Text::of(EN.fat_not_found))
    );
    assert_eq!(
        place.rename("a.psbt", "b.psbt"),
        Err(faraday_text::Text::fill(
            EN.files_already_there,
            &["b.psbt"]
        ))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Invariant: A vault longer than a frame is read from a folder stick in pieces and comes whole.
/// Regression: A vault longer than a frame is refused, or comes back cut at the frame's limit.
/// Owner: adapter
#[test]
fn a_vault_longer_than_a_frame_is_read_in_pieces_and_comes_whole() {
    let dir = folder("vault");
    let vault = vault_of(MAX_READ as usize + 5 * 1024 * 1024);
    std::fs::write(dir.join("big.kdbx"), &vault).unwrap();
    let mut place = FolderPlace(dir.clone());
    let mut got = Vec::new();
    loop {
        let (total, part) = read_part(&mut place, "big.kdbx", got.len() as u64, PIECE).unwrap();
        assert_eq!(total, vault.len() as u64);
        assert!(part.len() <= PIECE);
        got.extend_from_slice(&part);
        if got.len() as u64 >= total {
            break;
        }
    }
    assert!(got == vault, "the pieces are the file");
    assert!(read_whole(&mut place, "big.kdbx").unwrap() == vault);
    let _ = std::fs::remove_dir_all(dir);
}

/// Invariant: A file on a folder stick that is not a vault is not read past a frame, in pieces either.
/// Regression: Reading in pieces lets a long file that is not a vault past the frame's limit.
/// Owner: adapter
#[test]
fn a_file_that_is_not_a_vault_is_not_read_past_a_frame_in_pieces_either() {
    let dir = folder("plain");
    std::fs::write(dir.join("big.txt"), vec![7u8; MAX_READ as usize + 1]).unwrap();
    let mut place = FolderPlace(dir.clone());
    assert!(read_part(&mut place, "big.txt", 0, PIECE).is_err());
    assert!(read_whole(&mut place, "big.txt").is_err());
    let _ = std::fs::remove_dir_all(dir);
}
