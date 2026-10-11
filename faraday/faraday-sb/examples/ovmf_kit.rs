//! The files `faraday/tools/sb-ovmf-check.py` boots: test Secure Boot
//! keys made by this crate, their enrolment files where systemd-boot
//! enrols them from (`loader/keys/auto/{PK,KEK,db}.auth`), and an EFI
//! image three ways: signed with the db key, unsigned, and signed then
//! altered by one byte.
//!
//!     cargo run --release -p faraday-sb --example ovmf_kit -- OUT_DIR IMAGE.efi [windows]
//!
//! With `windows` the enrolment is the Windows-compatible policy's, with
//! Microsoft's KEK and db certificates beside the owner's; without it,
//! the owner's keys alone.
//!
//! The keys come from a fixed seed: they are test keys, the same on every
//! run, and nothing they sign is worth anything.

use std::path::Path;
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if !(3..=4).contains(&args.len()) {
        eprintln!("usage: ovmf_kit OUT_DIR IMAGE.efi [windows]");
        process::exit(2);
    }
    let policy = match args.get(3).map(String::as_str) {
        None => faraday_sb::Policy::OwnKeysOnly,
        Some("windows") => faraday_sb::Policy::Windows,
        Some(other) => fail(&format!("{other}: the policy is `windows` or nothing")),
    };
    let out = Path::new(&args[1]);
    let image = fs::read(&args[2]).unwrap_or_else(|e| fail(&format!("{}: {e}", args[2])));
    // 2026-10-07 00:00 UTC: the certificates' start and the updates'
    // timestamp.
    let now = 1_791_331_200;
    let keys =
        faraday_sb::make(&[0x5b; 32], "Faraday test", now).unwrap_or_else(|e| fail(e.reason()));
    let files = faraday_sb::enrolment(&keys, policy, now).unwrap_or_else(|e| fail(e.reason()));
    // Where systemd-boot enrols from by itself (`secure-boot-enroll`).
    let keys_dir = out.join("loader/keys/auto");
    fs::create_dir_all(&keys_dir).unwrap_or_else(|e| fail(&e.to_string()));
    for (name, bytes) in &files {
        let dir = if name.ends_with(".auth") {
            &keys_dir
        } else {
            out
        };
        fs::write(dir.join(name), bytes).unwrap_or_else(|e| fail(&e.to_string()));
    }
    let signed = faraday_sb::pe::sign(&image, &keys.db).unwrap_or_else(|e| fail(e.reason()));
    // One byte of the image's own body, well before the signature that
    // ends the file, so the hash it was signed over no longer holds.
    let mut altered = signed.clone();
    let at = image.len() / 2;
    altered[at] ^= 0x01;
    for (name, bytes) in [
        ("signed.efi", &signed),
        ("unsigned.efi", &image),
        ("altered.efi", &altered),
    ] {
        fs::write(out.join(name), bytes).unwrap_or_else(|e| fail(&e.to_string()));
    }
    println!("{}", out.display());
}

fn fail(why: &str) -> ! {
    eprintln!("ovmf_kit: {why}");
    process::exit(1);
}
