//! Which disks are Faraday sticks (`docs/STICK-IDENTITY.md` §5.1), read
//! from a table by the Starter's keep data and by `stick-boot.sh`, on the
//! same fixtures as `faraday-boot` and `faraday-grant` read.

use std::path::PathBuf;
use std::process::Command;

use faraday_starter::api::StickFault;
use faraday_starter_shell::gpt;

use crate::sticks::{
    self, BASIC, BOOT_FIRST, DATA_FIRST, ESP, IDS, IMAGE, NAMES, SECTOR, STICK, Sparse,
};

/// The last usable sector of a table `sticks::gpt` writes on a disk of
/// `STICK` bytes: 34 sectors from the end.
const LAST_USABLE: u64 = STICK / SECTOR - 34;

fn read_table(disk: &Sparse) -> Result<gpt::Table, StickFault> {
    gpt::read(
        &mut |at, buf| {
            if at
                .checked_add(buf.len() as u64)
                .is_none_or(|end| end > disk.len)
            {
                return Err(std::io::Error::other("past the end"));
            }
            disk.read_at(at, buf);
            Ok(())
        },
        disk.len,
    )
}

/// A stick of `STICK` bytes with a boot partition of `boot` sectors
/// and the data partitions given as (first, last) sectors.
fn stick_of(boot: u64, data: &[(u64, u64)]) -> Sparse {
    let mut s = Sparse::new(STICK);
    let mut parts = vec![(ESP, IDS[1], BOOT_FIRST, BOOT_FIRST + boot - 1, NAMES[0])];
    for &(first, last) in data {
        parts.push((BASIC, IDS[2], first, last, NAMES[1]));
    }
    sticks::gpt(&mut s, IDS[0], &parts);
    s
}

const BOOT: u64 = 131_072;

/// The fixtures: a name for the person reading a failure, the disk, and
/// whether it is a Faraday stick.
fn fixtures() -> Vec<(&'static str, Sparse, bool)> {
    vec![
        (
            "a stick written with dd",
            stick_of(BOOT, &[(DATA_FIRST, DATA_FIRST + 262_143)]),
            true,
        ),
        (
            "a stick made for its size",
            stick_of(BOOT, &[(DATA_FIRST, LAST_USABLE)]),
            true,
        ),
        (
            "a data partition under 128 MiB",
            stick_of(BOOT, &[(DATA_FIRST, DATA_FIRST + 262_142)]),
            false,
        ),
        (
            "a data partition starting elsewhere",
            stick_of(BOOT, &[(DATA_FIRST + 2048, DATA_FIRST + 2048 + 262_143)]),
            false,
        ),
        (
            "a data partition past the last usable sector",
            stick_of(BOOT, &[(DATA_FIRST, LAST_USABLE + 1)]),
            false,
        ),
        (
            "a third partition",
            stick_of(
                BOOT,
                &[
                    (DATA_FIRST, DATA_FIRST + 262_143),
                    (DATA_FIRST + 262_144, DATA_FIRST + 264_191),
                ],
            ),
            false,
        ),
        (
            "a boot partition of another size",
            stick_of(
                BOOT + 2048,
                &[(DATA_FIRST + 2048, DATA_FIRST + 2048 + 262_143)],
            ),
            false,
        ),
    ]
}

#[test]
fn a_stick_made_for_its_size_is_known_and_another_layout_is_not() {
    for (name, disk, known) in fixtures() {
        let table = read_table(&disk);
        if name == "a data partition past the last usable sector" {
            // A partition outside the usable area is no stick: the
            // table itself is refused.
            assert_eq!(table.err(), Some(StickFault::EntriesDamaged), "{name}");
        } else {
            let table = table.unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert_eq!(gpt::faraday_boot(&table, 0).is_ok(), known, "{name}");
        }
    }
}

#[test]
fn a_system_that_lists_the_partitions_says_the_same() {
    // Udisks2, diskmutil and Windows give the types, starts and lengths
    // in bytes and the disk's length.
    let part = |first: u64, last: u64, ty| gpt::Placed {
        type_guid: ty,
        offset: first * SECTOR,
        len: (last - first + 1) * SECTOR,
    };
    let end = gpt::usable_end(STICK);
    let boot = part(BOOT_FIRST, DATA_FIRST - 1, ESP);
    assert!(gpt::layout(&[boot, part(DATA_FIRST, DATA_FIRST + 262_143, BASIC)], end).is_ok());
    assert!(gpt::layout(&[boot, part(DATA_FIRST, LAST_USABLE, BASIC)], end).is_ok());
    assert!(gpt::layout(&[boot, part(DATA_FIRST, LAST_USABLE + 1, BASIC)], end).is_err());
    assert!(gpt::layout(&[boot, part(DATA_FIRST, DATA_FIRST + 262_142, BASIC)], end).is_err());
}

/// What Windows did to a stick bigger than the image (§2): the primary
/// header's backup place, last usable sector and entries' place moved
/// and its checksum worked out again, the entries copied unchanged just
/// before a new backup header at the disk's end.
fn windows_rewrote(image: &Sparse, first_usable: Option<u64>, disk: u64) -> Sparse {
    let crc = |h: &mut Vec<u8>| {
        h[16..20].fill(0);
        let c = faraday_crypto::crc32(&h[..92]);
        h[16..20].copy_from_slice(&c.to_le_bytes());
    };
    let last = disk / SECTOR - 1;
    let entries = image.bytes(2 * SECTOR, 128 * 128);
    let mut primary = image.bytes(SECTOR, 92);
    if let Some(fu) = first_usable {
        primary[40..48].copy_from_slice(&fu.to_le_bytes());
    }
    let fu = u64::from_le_bytes(primary[40..48].try_into().unwrap());
    primary[32..40].copy_from_slice(&last.to_le_bytes());
    primary[48..56].copy_from_slice(&(last - 33).to_le_bytes());
    primary[72..80].copy_from_slice(&(fu - 32).to_le_bytes());
    crc(&mut primary);
    let mut backup = primary.clone();
    backup[24..32].copy_from_slice(&last.to_le_bytes());
    backup[32..40].copy_from_slice(&1u64.to_le_bytes());
    backup[72..80].copy_from_slice(&(last - 32).to_le_bytes());
    crc(&mut backup);
    let mut s = Sparse::new(disk);
    s.put(0, &image.bytes(0, SECTOR as usize));
    s.put(SECTOR, &primary);
    s.put(2 * SECTOR, &entries);
    s.put((last - 32) * SECTOR, &entries);
    s.put(last * SECTOR, &backup);
    s
}

/// The stick image as genimage makes it, a table the size of the image.
fn image_table() -> Sparse {
    let mut image = Sparse::new(IMAGE);
    sticks::gpt(
        &mut image,
        IDS[0],
        &[
            (ESP, IDS[1], BOOT_FIRST, DATA_FIRST - 1, NAMES[0]),
            (BASIC, IDS[2], DATA_FIRST, DATA_FIRST + 262_143, NAMES[1]),
        ],
    );
    image
}

#[test]
fn a_stick_windows_rewrote_from_the_image_is_still_known() {
    let disk = windows_rewrote(&image_table(), None, 1 << 30);
    let table = read_table(&disk).expect("the rewritten table reads");
    assert!(gpt::faraday_boot(&table, 0).is_ok());
}

#[test]
fn a_stick_whose_primary_table_windows_broke_is_a_damaged_table_not_no_gpt() {
    // The image's first usable sector at 2048 puts the entries Windows
    // moves at sector 2016, where nothing is written.
    let disk = windows_rewrote(&image_table(), Some(2048), 1 << 30);
    assert_eq!(read_table(&disk).err(), Some(StickFault::EntriesChecksum));
    // A disk with no table at all is still said to have none.
    assert_eq!(
        read_table(&Sparse::new(STICK)).err(),
        Some(StickFault::NotGpt)
    );
}

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../image/stick-boot.sh")
}

/// `stick-boot.sh --dry-run` over `disk` as a regular file: whether it
/// takes it for a Faraday stick, or `None` where `sfdisk` is not here.
fn script_knows(name: &str, disk: &Sparse) -> Option<bool> {
    if Command::new("sfdisk").arg("--version").output().is_err() {
        if std::env::var_os("FARADAY_JUDGES_REQUIRED").is_some() {
            panic!("FARADAY_JUDGES_REQUIRED is set and sfdisk is not installed");
        }
        return None;
    }
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stick-layout");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("{}.img", name.replace(' ', "-")));
    disk.write(&file);
    let vfat = dir.join("boot.vfat");
    std::fs::write(&vfat, sticks::boot_sector(true, 1, b"EFI        ", 1)).unwrap();
    let out = Command::new("bash")
        .arg(script())
        .arg("--dry-run")
        .arg(&vfat)
        .arg(&file)
        .output()
        .expect("bash runs");
    let said =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    if out.status.success() {
        assert!(said.contains("Dry run"), "{name}: {said}");
    } else {
        assert!(said.contains("is not a Faraday stick"), "{name}: {said}");
    }
    let _ = std::fs::remove_file(&file);
    Some(out.status.success())
}

#[test]
fn the_developers_keep_data_knows_the_same_sticks() {
    for (name, disk, known) in fixtures() {
        if let Some(knew) = script_knows(name, &disk) {
            assert_eq!(knew, known, "{name}");
        }
    }
    let disk = windows_rewrote(&image_table(), None, 1 << 30);
    if let Some(knew) = script_knows("windows rewrote", &disk) {
        assert!(knew, "a stick Windows rewrote");
    }
}

/// A Faraday stick whose primary header says its entries are at
/// `entries_lba` (copied there), with `count` entries, checksum worked
/// out again.
fn entries_moved(entries_lba: u64, count: u32) -> Sparse {
    let mut s = stick_of(BOOT, &[(DATA_FIRST, DATA_FIRST + 262_143)]);
    let entries = s.bytes(2 * SECTOR, 128 * 128);
    let mut h = s.bytes(SECTOR, 92);
    h[72..80].copy_from_slice(&entries_lba.to_le_bytes());
    h[80..84].copy_from_slice(&count.to_le_bytes());
    h[16..20].fill(0);
    let crc = faraday_crypto::crc32(&h);
    h[16..20].copy_from_slice(&crc.to_le_bytes());
    s.put(SECTOR, &h);
    s.put(entries_lba * SECTOR, &entries);
    s
}

#[test]
fn a_table_with_its_entries_inside_the_boot_partition_is_refused() {
    let disk = entries_moved(BOOT_FIRST + 100, 128);
    assert_eq!(read_table(&disk).err(), Some(StickFault::EntriesDamaged));
}

#[test]
fn a_table_with_its_entries_at_the_disks_end_is_refused() {
    let disk = entries_moved(STICK / SECTOR - 40, 128);
    assert_eq!(read_table(&disk).err(), Some(StickFault::EntriesDamaged));
}

/// `disk`'s primary header after `edit`, its checksum worked out again.
fn header_edited(mut disk: Sparse, edit: impl FnOnce(&mut [u8])) -> Sparse {
    let mut h = disk.bytes(SECTOR, 92);
    edit(&mut h);
    h[16..20].fill(0);
    let crc = faraday_crypto::crc32(&h);
    h[16..20].copy_from_slice(&crc.to_le_bytes());
    disk.put(SECTOR, &h);
    disk
}

#[test]
fn a_header_whose_entries_run_nearly_to_the_disks_end_is_refused_before_they_are_read() {
    // 1024 entries of 4096 bytes at the last 8200 sectors: the old
    // reader took this table and read the whole disk but its last sectors.
    let lba = STICK / SECTOR - 8200;
    let sums = faraday_crypto::crc32(&vec![0u8; 1024 * 4096]);
    let disk = header_edited(stick_of(BOOT, &[(DATA_FIRST, DATA_FIRST + 262_143)]), |h| {
        h[72..80].copy_from_slice(&lba.to_le_bytes());
        h[80..84].copy_from_slice(&1024u32.to_le_bytes());
        h[84..88].copy_from_slice(&4096u32.to_le_bytes());
        h[88..92].copy_from_slice(&sums.to_le_bytes());
    });
    let mut most = 0;
    let got = gpt::read(
        &mut |at, buf| {
            most = most.max(buf.len());
            disk.read_at(at, buf);
            Ok(())
        },
        disk.len,
    );
    assert_eq!(got.err(), Some(StickFault::EntriesDamaged));
    assert!(most <= 1 << 20, "a read of {most} bytes");
}

#[test]
fn a_table_whose_backup_header_lies_inside_a_partition_is_refused() {
    for alternate in [
        BOOT_FIRST + 100,
        DATA_FIRST + 5,
        LAST_USABLE + 32,
        STICK / SECTOR,
    ] {
        let disk = header_edited(stick_of(BOOT, &[(DATA_FIRST, LAST_USABLE)]), |h| {
            h[32..40].copy_from_slice(&alternate.to_le_bytes());
        });
        assert_eq!(
            read_table(&disk).err(),
            Some(StickFault::EntriesDamaged),
            "backup header at {alternate}"
        );
    }
}

#[test]
fn a_table_leaves_the_backups_room_at_the_disks_end_exactly() {
    // 128 entries of 128 bytes are 32 sectors: the last usable sector
    // may be the 34th from the end and no later.
    let end = STICK / SECTOR;
    for (last_usable, ok) in [(end - 34, true), (end - 33, false)] {
        let disk = header_edited(stick_of(BOOT, &[(DATA_FIRST, DATA_FIRST + 262_143)]), |h| {
            h[48..56].copy_from_slice(&last_usable.to_le_bytes());
        });
        let got = read_table(&disk);
        if ok {
            assert!(got.is_ok(), "last usable {last_usable}: {:?}", got.err());
        } else {
            assert_eq!(got.err(), Some(StickFault::EntriesDamaged));
        }
    }
}

#[test]
fn a_table_with_a_partition_before_the_first_usable_sector_is_refused() {
    let disk = header_edited(stick_of(BOOT, &[(DATA_FIRST, LAST_USABLE)]), |h| {
        h[40..48].copy_from_slice(&(BOOT_FIRST + 1).to_le_bytes());
    });
    assert_eq!(read_table(&disk).err(), Some(StickFault::EntriesDamaged));
}

#[test]
fn a_table_with_a_partition_outside_the_usable_area_is_refused() {
    let disk = stick_of(BOOT, &[(DATA_FIRST, STICK / SECTOR - 1)]);
    assert_eq!(read_table(&disk).err(), Some(StickFault::EntriesDamaged));
}
