//! A vault longer than a frame is read from a stick in pieces; any other
//! file that long is refused, and the frame keeps its limit.

use faraday_files::proto::{MAX_FRAME, PIECE, Response};
use faraday_files::{Dir, MAX_READ, read_part, read_whole};

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

#[test]
fn a_vault_longer_than_a_frame_is_read_in_pieces_and_comes_whole() {
    let dir = folder("vault");
    let vault = vault_of(MAX_READ as usize + 5 * 1024 * 1024);
    std::fs::write(dir.join("big.kdbx"), &vault).unwrap();
    let mut place = Dir(dir.clone());
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

#[test]
fn a_file_that_is_not_a_vault_is_not_read_past_a_frame_in_pieces_either() {
    let dir = folder("plain");
    std::fs::write(dir.join("big.txt"), vec![7u8; MAX_READ as usize + 1]).unwrap();
    let mut place = Dir(dir.clone());
    assert!(read_part(&mut place, "big.txt", 0, PIECE).is_err());
    assert!(read_whole(&mut place, "big.txt").is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_piece_fits_a_frame_whose_limit_is_unchanged() {
    let piece = Response::Part {
        total: 1 << 28,
        bytes: vec![0; PIECE],
    }
    .encode();
    assert!(piece.len() as u32 <= MAX_FRAME);
}
