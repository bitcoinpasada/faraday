//! Finding the release's images in the Downloads folder and among dropped
//! files (`docs/IMAGES.md` §5): the right file found with its hardware, a
//! file with the right name and the wrong contents named and not offered,
//! last release's file named with the version this Starter needs, a file
//! the list does not name left unread, and a file replaced after it was
//! found hashed again.

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use faraday_crypto::work::{Compute, Work};
use faraday_payload::{ImageList, sha256_of};
use faraday_starter::api::ImageFinding;
use faraday_starter_shell::finder::{Memo, find};

use crate::writer::tmp;

const PI: &str = "faraday-pi-0.3.0.img.xz";
const STICK: &str = "faraday-stick-0.3.0-x86_64-uefi.img.xz";
const COMMIT: &str = "89abcdef0123456789abcdef0123456789abcdef";

thread_local! {
    static HASHED: Cell<usize> = const { Cell::new(0) };
}

fn count(w: Work) {
    if matches!(w, Work::FileHash) {
        HASHED.set(HASHED.get() + 1);
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: the finder's tests count the files hashed"
)]
fn compute() -> Compute {
    Compute::new(count)
}

fn hashed() -> usize {
    HASHED.get()
}

fn sum(bytes: &[u8]) -> String {
    sha256_of(bytes).expect("a slice reads").to_string()
}

/// The release's image list for a fake Pi image and a fake stick image.
fn list(pi: &[u8], stick: &[u8]) -> ImageList {
    let text = format!(
        "# Faraday 0.3.0 images, commit {COMMIT}\n\
         {STICK}\t{}\t{}\tx86_64-uefi\tefi-framebuffer\tPC\n\
         {PI}\t{}\t{}\tpi3\tany\tPi 3B, Pi 3B+, Pi Zero 2 W\n",
        sum(stick),
        sum(b"inner stick"),
        sum(pi),
        sum(b"inner pi"),
    );
    ImageList::parse(text.as_bytes()).expect("a good list")
}

/// A new, empty Downloads folder.
fn downloads(name: &str) -> PathBuf {
    let dir = tmp(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a folder");
    dir
}

fn findings(
    list: &ImageList,
    dir: &Path,
    dropped: &[PathBuf],
    memo: &mut Memo,
) -> Vec<ImageFinding> {
    find(&compute(), list, dir, dropped, memo)
}

#[test]
fn the_right_file_is_found_with_its_hardware() {
    let (pi, stick) = (b"a pi image".as_slice(), b"a stick image".as_slice());
    let list = list(pi, stick);
    let dir = downloads("finder-found");
    fs::write(dir.join(PI), pi).expect("write");
    let found = findings(&list, &dir, &[], &mut Memo::default());
    let [ImageFinding::Found(image)] = &found[..] else {
        panic!("{found:?}");
    };
    assert_eq!(image.name, PI);
    assert_eq!(image.path, dir.join(PI).display().to_string());
    assert_eq!((image.board.as_str(), image.panel.as_str()), ("pi3", "any"));
    assert_eq!(image.starts, ["Pi 3B", "Pi 3B+", "Pi Zero 2 W"]);
    // The sums it carries are the list's, for the writer to check again.
    assert_eq!(image.xz_sha256, sum(pi));
    assert_eq!(image.img_sha256, sum(b"inner pi"));
}

#[test]
fn a_file_dropped_on_the_window_is_found_too() {
    let (pi, stick) = (b"a pi image".as_slice(), b"a stick image".as_slice());
    let list = list(pi, stick);
    let dir = downloads("finder-dropped-in");
    let elsewhere = downloads("finder-dropped-from");
    fs::write(elsewhere.join(STICK), stick).expect("write");
    let found = findings(&list, &dir, &[elsewhere.join(STICK)], &mut Memo::default());
    assert!(matches!(&found[..], [ImageFinding::Found(i)] if i.name == STICK));
}

#[test]
fn a_file_with_the_wrong_contents_is_named_and_not_offered() {
    let (pi, stick) = (b"a pi image".as_slice(), b"a stick image".as_slice());
    let list = list(pi, stick);
    let dir = downloads("finder-wrong");
    fs::write(dir.join(PI), b"a pi imagf").expect("write");
    let found = findings(&list, &dir, &[], &mut Memo::default());
    assert_eq!(
        found,
        [ImageFinding::Mismatch {
            path: dir.join(PI).display().to_string(),
            name: PI.to_owned()
        }]
    );
}

#[test]
fn last_releases_file_is_named_with_the_version_needed() {
    let (pi, stick) = (b"a pi image".as_slice(), b"a stick image".as_slice());
    let list = list(pi, stick);
    let dir = downloads("finder-other");
    let old = "faraday-pi-0.2.1.img.xz";
    fs::write(dir.join(old), b"last release").expect("write");
    let before = hashed();
    let found = findings(&list, &dir, &[], &mut Memo::default());
    assert_eq!(
        found,
        [ImageFinding::OtherRelease {
            path: dir.join(old).display().to_string(),
            name: old.to_owned(),
            needs: "0.3.0".to_owned()
        }]
    );
    assert_eq!(hashed(), before, "another release's file is not read");
}

#[test]
fn a_large_file_the_list_does_not_name_is_not_hashed() {
    let (pi, stick) = (b"a pi image".as_slice(), b"a stick image".as_slice());
    let list = list(pi, stick);
    let dir = downloads("finder-unrelated");
    fs::write(dir.join("holiday-video.mp4"), vec![7u8; 8 << 20]).expect("write");
    fs::write(dir.join("faraday-pi-0.3.0.img"), pi).expect("write");
    fs::create_dir(dir.join(PI)).expect("a folder named like an image");
    let before = hashed();
    let found = findings(&list, &dir, &[], &mut Memo::default());
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(hashed(), before);
}

#[test]
fn a_file_is_hashed_once_and_again_when_it_is_replaced() {
    let (pi, stick) = (b"a pi image".as_slice(), b"a stick image".as_slice());
    let list = list(pi, stick);
    let dir = downloads("finder-replaced");
    let path = dir.join(PI);
    fs::write(&path, pi).expect("write");
    let mut memo = Memo::default();
    let before = hashed();
    assert!(matches!(
        &findings(&list, &dir, &[], &mut memo)[..],
        [ImageFinding::Found(_)]
    ));
    assert!(matches!(
        &findings(&list, &dir, &[], &mut memo)[..],
        [ImageFinding::Found(_)]
    ));
    assert_eq!(hashed(), before + 1, "the second look reads nothing");

    // The same size, written later.
    fs::write(&path, b"a pi imagf").expect("write");
    let later = SystemTime::now() + Duration::from_secs(60);
    fs::File::options()
        .write(true)
        .open(&path)
        .expect("open")
        .set_modified(later)
        .expect("mtime");
    let found = findings(&list, &dir, &[], &mut memo);
    assert!(
        matches!(&found[..], [ImageFinding::Mismatch { .. }]),
        "{found:?}"
    );
    assert_eq!(hashed(), before + 2);
}
