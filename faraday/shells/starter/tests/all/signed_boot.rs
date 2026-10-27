//! A boot file the person signed with their own Secure Boot db key, written
//! in place of the release's `EFI/BOOT/BOOTX64.EFI` (`docs/STARTER.md`
//! §4.5, `docs/DECISIONS.md` F133): taken only when it is the release's own
//! program and carries a signature, refused by name otherwise, checked
//! when it is chosen and again by the writer (the elevated copy on
//! Windows), and written with the rest of the boot partition the release's.

use std::io::Cursor;
use std::path::PathBuf;

use faraday_fat::Volume;
use faraday_payload::ImageKind;
use faraday_sb::rng::Stream;
use faraday_starter::api::{BootFile, Mode, Reason, Source as Confirmed, WriteOrder};
use faraday_starter_shell::payload::Payload;
use faraday_starter_shell::signed::{self, BOOT_PATH};
use faraday_starter_shell::writer::Job;

use crate::writer::{
    BOOT_FIRST, DATA_FIRST, FileDisk, SECTOR, run, stick_image, tmp, untouched_outside,
};

const NOW: u64 = 1_791_000_000;
const NAME: &str = "faraday-stick-0.3.0-x86_64-uefi.img.xz";
const COMMIT: &str = "89abcdef0123456789abcdef0123456789abcdef";
const BUILD: &[u8] = b"0.3.0+89abcdef0123\n";

/// A PE32+ EFI program: headers, then one section of `code_len` bytes of
/// code drawn from `seed`, as a kernel's boot file is laid out.
fn program(seed: u8, code_len: usize) -> Vec<u8> {
    let raw = code_len.div_ceil(0x200) * 0x200;
    let mut pe = vec![0u8; 0x200 + raw];
    let put16 = |pe: &mut [u8], at: usize, v: u16| pe[at..at + 2].copy_from_slice(&v.to_le_bytes());
    let put32 = |pe: &mut [u8], at: usize, v: u32| pe[at..at + 4].copy_from_slice(&v.to_le_bytes());
    pe[0..2].copy_from_slice(b"MZ");
    put32(&mut pe, 0x3c, 0x40);
    pe[0x40..0x44].copy_from_slice(b"PE\0\0");
    let coff = 0x44;
    put16(&mut pe, coff, 0x8664);
    put16(&mut pe, coff + 2, 1);
    put16(&mut pe, coff + 16, 240);
    put16(&mut pe, coff + 18, 0x22);
    let opt = coff + 20;
    put16(&mut pe, opt, 0x20b);
    put32(&mut pe, opt + 16, 0x1000);
    put32(&mut pe, opt + 32, 0x1000);
    put32(&mut pe, opt + 36, 0x200);
    put32(
        &mut pe,
        opt + 56,
        (0x1000 + raw.div_ceil(0x1000) * 0x1000) as u32,
    );
    put32(&mut pe, opt + 60, 0x200);
    put16(&mut pe, opt + 68, 10);
    put32(&mut pe, opt + 108, 16);
    let sec = opt + 240;
    pe[sec..sec + 5].copy_from_slice(b".text");
    put32(&mut pe, sec + 8, code_len as u32);
    put32(&mut pe, sec + 12, 0x1000);
    put32(&mut pe, sec + 16, raw as u32);
    put32(&mut pe, sec + 20, 0x200);
    put32(&mut pe, sec + 36, 0x6000_0020);
    for (i, b) in pe[0x200..0x200 + code_len].iter_mut().enumerate() {
        *b = (i as u32)
            .wrapping_mul(2_654_435_761)
            .wrapping_add(u32::from(seed)) as u8;
    }
    pe
}

/// The program this test's release carries.
fn release_program() -> Vec<u8> {
    program(1, 30_000)
}

/// A FAT32 boot partition of `len` bytes labelled FARADAYBOOT, as
/// `mkfs.fat` and `mcopy` make the stick's (`genimage.cfg`): 512-byte
/// clusters, the root, `EFI` and `EFI/BOOT` folders first, then
/// `BUILD.TXT`, then `BOOTX64.EFI` in a row, the rest free.
fn fat_boot(len: usize, efi: &[u8]) -> Vec<u8> {
    let mut v = vec![0u8; len];
    let sectors = (len / 512) as u32;
    let reserved = 32u32;
    let fat_sectors = (sectors / 128) + 1;
    let put16 = |v: &mut [u8], at: usize, x: u16| v[at..at + 2].copy_from_slice(&x.to_le_bytes());
    let put32 = |v: &mut [u8], at: usize, x: u32| v[at..at + 4].copy_from_slice(&x.to_le_bytes());
    v[0..3].copy_from_slice(&[0xEB, 0x58, 0x90]);
    v[3..11].copy_from_slice(b"mkfs.fat");
    put16(&mut v, 11, 512);
    v[13] = 1;
    put16(&mut v, 14, reserved as u16);
    v[16] = 2;
    v[21] = 0xF8;
    put32(&mut v, 32, sectors);
    put32(&mut v, 36, fat_sectors);
    put32(&mut v, 44, 2);
    v[64] = 0x80;
    v[66] = 0x29;
    put32(&mut v, 67, 0x1234_5678);
    v[71..82].copy_from_slice(b"FARADAYBOOT");
    v[82..90].copy_from_slice(b"FAT32   ");
    v[510] = 0x55;
    v[511] = 0xAA;
    let data = (reserved + 2 * fat_sectors) as usize * 512;
    let cluster = |c: u32| data + (c as usize - 2) * 512;
    // The chains: 2 the root, 3 EFI, 4 BOOT, 5 BUILD.TXT, then the file.
    let n = efi.len().div_ceil(512) as u32;
    let mut fat = vec![0u32; 6 + n as usize];
    fat[0] = 0x0FFF_FFF8;
    for c in [1usize, 2, 3, 4, 5] {
        fat[c] = 0x0FFF_FFFF;
    }
    for k in 0..n {
        fat[6 + k as usize] = if k + 1 == n { 0x0FFF_FFFF } else { 7 + k };
    }
    for copy in 0..2 {
        let at = (reserved + copy * fat_sectors) as usize * 512;
        for (i, e) in fat.iter().enumerate() {
            put32(&mut v, at + 4 * i, *e);
        }
    }
    let entry = |v: &mut [u8], at: usize, name: &[u8; 11], attr: u8, first: u32, size: u32| {
        v[at..at + 11].copy_from_slice(name);
        v[at + 11] = attr;
        v[at + 20..at + 22].copy_from_slice(&((first >> 16) as u16).to_le_bytes());
        v[at + 26..at + 28].copy_from_slice(&(first as u16).to_le_bytes());
        v[at + 28..at + 32].copy_from_slice(&size.to_le_bytes());
    };
    entry(&mut v, cluster(2), b"FARADAYBOOT", 0x08, 0, 0);
    entry(&mut v, cluster(2) + 32, b"EFI        ", 0x10, 3, 0);
    entry(
        &mut v,
        cluster(2) + 64,
        b"BUILD   TXT",
        0x20,
        5,
        BUILD.len() as u32,
    );
    entry(&mut v, cluster(3), b".          ", 0x10, 3, 0);
    entry(&mut v, cluster(3) + 32, b"..         ", 0x10, 0, 0);
    entry(&mut v, cluster(3) + 64, b"BOOT       ", 0x10, 4, 0);
    entry(&mut v, cluster(4), b".          ", 0x10, 4, 0);
    entry(&mut v, cluster(4) + 32, b"..         ", 0x10, 3, 0);
    entry(
        &mut v,
        cluster(4) + 64,
        b"BOOTX64 EFI",
        0x20,
        6,
        efi.len() as u32,
    );
    v[cluster(5)..cluster(5) + BUILD.len()].copy_from_slice(BUILD);
    v[cluster(6)..cluster(6) + efi.len()].copy_from_slice(efi);
    v
}

/// The release's stick image: Faraday's partition table, its boot
/// partition holding `efi`, its data partition empty.
fn release_image(efi: &[u8]) -> Vec<u8> {
    let mut img = stick_image(faraday_starter_shell::gpt::DISK_UUID, 0);
    let (from, to) = boot_range();
    img[from..to].copy_from_slice(&fat_boot(to - from, efi));
    img
}

fn boot_range() -> (usize, usize) {
    (
        (BOOT_FIRST * SECTOR) as usize,
        (DATA_FIRST * SECTOR) as usize,
    )
}

/// `data` in an `.xz` whose LZMA2 chunks are stored, not compressed, as
/// the xz format allows: what a release's payload file is, made here
/// without an encoder.
fn stored_xz(data: &[u8]) -> Vec<u8> {
    let crc = faraday_crypto::crc32;
    let varint = |out: &mut Vec<u8>, mut n: u64| {
        while n >= 0x80 {
            out.push((n as u8) | 0x80);
            n >>= 7;
        }
        out.push(n as u8);
    };
    let mut x = vec![0xFD, b'7', b'z', b'X', b'Z', 0, 0, 0];
    x.extend_from_slice(&crc(&[0, 0]).to_le_bytes());
    // The block: its header (LZMA2, a 4 MiB dictionary), the chunks.
    let mut header = vec![2u8, 0x00, 0x21, 0x01, 0x14, 0, 0, 0];
    header.extend_from_slice(&crc(&header).to_le_bytes());
    x.extend_from_slice(&header);
    let mut body = 0u64;
    for (k, chunk) in data.chunks(1 << 16).enumerate() {
        x.push(if k == 0 { 0x01 } else { 0x02 });
        x.extend_from_slice(&((chunk.len() - 1) as u16).to_be_bytes());
        x.extend_from_slice(chunk);
        body += 3 + chunk.len() as u64;
    }
    x.push(0x00);
    body += 1;
    while !x.len().is_multiple_of(4) {
        x.push(0);
    }
    let mut index = vec![0u8];
    varint(&mut index, 1);
    varint(&mut index, header.len() as u64 + body);
    varint(&mut index, data.len() as u64);
    while !index.len().is_multiple_of(4) {
        index.push(0);
    }
    let sum = crc(&index);
    index.extend_from_slice(&sum.to_le_bytes());
    x.extend_from_slice(&index);
    let mut footer = ((index.len() / 4 - 1) as u32).to_le_bytes().to_vec();
    footer.extend_from_slice(&[0, 0]);
    x.extend_from_slice(&crc(&footer).to_le_bytes());
    x.extend_from_slice(&footer);
    x.extend_from_slice(b"YZ");
    x
}

/// The SHA-256 and size of `bytes`, hex.
fn sum(bytes: &[u8]) -> (String, u64) {
    let s = faraday_payload::hash_image(Cursor::new(bytes), ImageKind::Plain).expect("hashed");
    (s.sha256.to_string(), s.size)
}

/// The Starter's payload carrying the release image of `efi`, and the
/// order's source for it.
fn release(efi: &[u8]) -> (Job, Confirmed) {
    let img = release_image(efi);
    let xz = stored_xz(&img);
    let (xz_sha, _) = sum(&xz);
    let (img_sha, size) = sum(&img);
    let manifest = format!(
        "# Faraday Starter payload 0.3.0, commit {COMMIT}\n{NAME}\t{xz_sha}\tfaraday-stick-0.3.0-x86_64-uefi.img\t{img_sha}\t{size}\t0.3.0+89abcdef0123\n"
    );
    let payload =
        Payload::from_parts(manifest.as_bytes(), vec![(NAME.to_string(), xz)]).expect("a payload");
    let job = payload.job(NAME, &img_sha).expect("carried");
    let source = Confirmed::Payload {
        name: NAME.to_string(),
        inner_sha256: img_sha,
    };
    (job, source)
}

/// A db key and its certificate, made as Faraday's Secure Boot tools
/// make them, and `pe` signed with it.
fn signed_by_test_key(pe: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let keys = faraday_sb::make(&[0x57; 32], "Test", NOW).expect("keys");
    let signed =
        faraday_sb::pe::sign(&mut Stream::new(&[0x58; 32], 0), pe, &keys.db).expect("signed");
    (signed, keys.db.cert.clone())
}

fn file(name: &str, bytes: &[u8]) -> PathBuf {
    let path = tmp(&format!("signed-{name}"));
    std::fs::write(&path, bytes).expect("written");
    path
}

/// A stick that holds an older Faraday and the person's data.
fn old_stick(name: &str) -> (PathBuf, Vec<u8>) {
    let path = tmp(&format!("signed-stick-{name}"));
    let mut before = stick_image(faraday_starter_shell::gpt::DISK_UUID, 0xDA);
    let (from, to) = boot_range();
    before[from..to].copy_from_slice(&fat_boot(to - from, &program(9, 20_000)));
    std::fs::write(&path, &before).expect("a stick");
    (path, before)
}

fn keep_data(disk: &FileDisk, source: Confirmed, boot_file: Option<BootFile>) -> WriteOrder {
    WriteOrder {
        boot_file,
        ..disk.order(source, Mode::KeepData)
    }
}

/// The bytes of a FAT32 boot partition's FATs, the `EFI/BOOT` folder
/// and the clusters `BOOTX64.EFI` holds, as `fat_boot` lays them out:
/// where a written-over boot file may differ from the release's.
fn boot_file_places(partition_len: usize, efi_len: usize) -> Vec<(usize, usize)> {
    let sectors = partition_len / 512;
    let fat_sectors = sectors / 128 + 1;
    let data = (32 + 2 * fat_sectors) * 512;
    let cluster = |c: usize| data + (c - 2) * 512;
    vec![
        (32 * 512, data),
        (cluster(4), cluster(5)),
        (cluster(6), cluster(6) + efi_len.div_ceil(512) * 512),
    ]
}

#[test]
fn a_release_boot_file_signed_with_ones_own_db_key_is_written_in_place_of_the_releases() {
    let release_efi = release_program();
    let (job, source) = release(&release_efi);
    let (signed, cert) = signed_by_test_key(&release_efi);
    let path = file("own.efi", &signed);

    // Chosen: checked against the release and its signer shown.
    let (chosen, signer) = signed::choose(&path, &job).expect("taken");
    assert_eq!(chosen.path, path.display().to_string());
    assert_eq!(chosen.sha256, sum(&signed).0);
    assert_eq!(signer.subject, "Test db");
    assert_eq!(signer.fingerprint, sum(&cert).0);

    // Written over a stick, keeping its data.
    let (stick, before) = old_stick("own");
    let mut disk = FileDisk::new(&stick);
    let order = keep_data(&disk, source.clone(), Some(chosen.clone()));
    let (r, _) = run(&order, &job, &mut disk);
    assert_eq!(r, Ok(()));
    let after = std::fs::read(&stick).expect("read");
    let (from, to) = boot_range();
    untouched_outside(&before, &after, from, to);
    let written = after[from..to].to_vec();
    let mut v = Volume::open(written.clone()).expect("a FAT32 boot partition");
    assert_eq!(v.read_in(&BOOT_PATH, 64 << 20).expect("there"), signed);
    assert_eq!(v.read("BUILD.TXT", 1 << 20).expect("there"), BUILD);
    // Everything but the FATs, the folder's entry and the file's
    // clusters is the release's, byte for byte.
    let release = &release_image(&release_efi)[from..to];
    let mut at = 0;
    for (a, b) in boot_file_places(to - from, signed.len()) {
        assert_eq!(written[at..a], release[at..a], "bytes {at}..{a} changed");
        at = b;
    }
    assert_eq!(written[at..], release[at..], "bytes after the file changed");

    // Written whole: the same boot partition, the release's table.
    let blank = tmp("signed-blank");
    std::fs::write(&blank, vec![0xEE; release_image(&release_efi).len()]).expect("a disk");
    let mut disk = FileDisk::new(&blank);
    let order = WriteOrder {
        boot_file: Some(chosen),
        ..disk.order(source, Mode::Whole)
    };
    assert_eq!(run(&order, &job, &mut disk).0, Ok(()));
    let whole = std::fs::read(&blank).expect("read");
    assert_eq!(whole[from..to], written[..]);
    untouched_outside(&release_image(&release_efi), &whole, from, to);
}

#[test]
fn the_same_program_with_one_code_byte_changed_then_signed_is_refused() {
    let release_efi = release_program();
    let (job, source) = release(&release_efi);
    let mut changed = release_efi.clone();
    changed[0x200 + 1_000] ^= 0x01;
    let (signed, _) = signed_by_test_key(&changed);
    let path = file("changed.efi", &signed);
    assert_eq!(signed::choose(&path, &job), Err(Reason::BootFileOther));
    // The writer checks it again (the elevated copy on Windows) and
    // writes nothing.
    let (stick, before) = old_stick("changed");
    let mut disk = FileDisk::new(&stick);
    let order = keep_data(
        &disk,
        source,
        Some(BootFile {
            path: path.display().to_string(),
            sha256: sum(&signed).0,
        }),
    );
    assert_eq!(run(&order, &job, &mut disk).0, Err(Reason::BootFileOther));
    assert_eq!(std::fs::read(&stick).expect("read"), before);
    assert_eq!(disk.calls, ["identity"]);
}

#[test]
fn the_releases_own_boot_file_unsigned_is_refused_as_one_signed() {
    let release_efi = release_program();
    let (job, source) = release(&release_efi);
    let path = file("unsigned.efi", &release_efi);
    assert_eq!(signed::choose(&path, &job), Err(Reason::BootFileUnsigned));
    let (stick, before) = old_stick("unsigned");
    let mut disk = FileDisk::new(&stick);
    let order = keep_data(
        &disk,
        source,
        Some(BootFile {
            path: path.display().to_string(),
            sha256: sum(&release_efi).0,
        }),
    );
    assert_eq!(
        run(&order, &job, &mut disk).0,
        Err(Reason::BootFileUnsigned)
    );
    assert_eq!(std::fs::read(&stick).expect("read"), before);
}

#[test]
fn a_signed_boot_file_of_another_release_is_refused() {
    let (job, _) = release(&release_program());
    let (signed, _) = signed_by_test_key(&program(2, 31_000));
    let path = file("other.efi", &signed);
    assert_eq!(signed::choose(&path, &job), Err(Reason::BootFileOther));
    let path = file("not-efi.efi", b"a text file, not a boot file\n");
    assert_eq!(signed::choose(&path, &job), Err(Reason::BootFileOther));
}

/// The security directory entry of [`program`]'s headers: offset, size.
const CERT_DIR: usize = 0x44 + 20 + 112 + 4 * 8;

fn set_dir(image: &mut [u8], at: usize, size: usize) {
    image[CERT_DIR..CERT_DIR + 4].copy_from_slice(&(at as u32).to_le_bytes());
    image[CERT_DIR + 4..CERT_DIR + 8].copy_from_slice(&(size as u32).to_le_bytes());
}

fn dir(image: &[u8]) -> (usize, usize) {
    let n = |at: usize| u32::from_le_bytes(image[at..at + 4].try_into().expect("four")) as usize;
    (n(CERT_DIR), n(CERT_DIR + 4))
}

#[test]
fn a_signed_boot_file_with_more_than_one_signature_at_its_end_is_refused() {
    let release_efi = release_program();
    let (job, source) = release(&release_efi);
    let (signed, _) = signed_by_test_key(&release_efi);
    let (at, size) = dir(&signed);
    // Bytes appended after the table.
    let mut appended = signed.clone();
    appended.extend_from_slice(b"anything at all\0");
    // Two signatures: the test db key's and another key's.
    let other = faraday_sb::make(&[0x59; 32], "Other", NOW).expect("keys");
    let second = faraday_sb::pe::sign(&mut Stream::new(&[0x5A; 32], 0), &release_efi, &other.db)
        .expect("signed");
    let (at2, size2) = dir(&second);
    let mut two = signed.clone();
    two.extend_from_slice(&second[at2..at2 + size2]);
    set_dir(&mut two, at, size + size2);
    // The table not where the program ends.
    let mut moved = signed[..at].to_vec();
    moved.extend_from_slice(&[0u8; 8]);
    moved.extend_from_slice(&signed[at..]);
    set_dir(&mut moved, at + 8, size);
    for (name, bytes, why) in [
        ("appended.efi", appended, Reason::BootFileLayout),
        ("two.efi", two, Reason::BootFileLayout),
        ("moved.efi", moved, Reason::BootFileOther),
    ] {
        let path = file(name, &bytes);
        assert_eq!(signed::choose(&path, &job), Err(why.clone()), "{name}");
        let (stick, before) = old_stick(name);
        let mut disk = FileDisk::new(&stick);
        let order = keep_data(
            &disk,
            source.clone(),
            Some(BootFile {
                path: path.display().to_string(),
                sha256: sum(&bytes).0,
            }),
        );
        assert_eq!(run(&order, &job, &mut disk).0, Err(why), "{name}");
        assert_eq!(std::fs::read(&stick).expect("read"), before, "{name}");
    }
}

#[test]
fn a_signature_that_does_not_hold_is_refused_as_such() {
    let release_efi = release_program();
    let (job, _) = release(&release_efi);
    let (mut signed, _) = signed_by_test_key(&release_efi);
    // A byte of the signature's own value changed.
    let last = signed.len() - 40;
    signed[last] ^= 0x01;
    let path = file("broken.efi", &signed);
    assert_eq!(signed::choose(&path, &job), Err(Reason::BootFileBroken));
}

#[test]
fn a_certificate_name_is_shown_without_control_or_direction_characters() {
    assert_eq!(
        signed::shown_name("Test\u{202E}bd\u{2066} db\n\u{0007}"),
        "Testbd db"
    );
    assert_eq!(signed::shown_name(&"x".repeat(200)).len(), 64);
}

#[test]
fn a_boot_file_changed_after_it_was_chosen_is_not_written() {
    let release_efi = release_program();
    let (job, source) = release(&release_efi);
    let (signed, _) = signed_by_test_key(&release_efi);
    let path = file("swapped.efi", &signed);
    let (chosen, _) = signed::choose(&path, &job).expect("taken");
    // Between Confirm and the write, another file under its name.
    let (other, _) = signed_by_test_key(&program(3, 30_000));
    std::fs::write(&path, &other).expect("written");
    let (stick, before) = old_stick("swapped");
    let mut disk = FileDisk::new(&stick);
    let order = keep_data(&disk, source, Some(chosen));
    assert_eq!(
        run(&order, &job, &mut disk).0,
        Err(Reason::BootFileChanged(path.display().to_string()))
    );
    assert_eq!(std::fs::read(&stick).expect("read"), before);
}

#[test]
fn without_a_signed_boot_file_keep_data_writes_the_releases_boot_partition() {
    let release_efi = release_program();
    let (job, source) = release(&release_efi);
    let (stick, before) = old_stick("default");
    let mut disk = FileDisk::new(&stick);
    let order = keep_data(&disk, source, None);
    assert_eq!(run(&order, &job, &mut disk).0, Ok(()));
    let after = std::fs::read(&stick).expect("read");
    let (from, to) = boot_range();
    untouched_outside(&before, &after, from, to);
    assert_eq!(after[from..to], release_image(&release_efi)[from..to]);
}
