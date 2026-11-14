//! A stick pulled in the middle of a vault write-back (`docs/VAULT.md`
//! §6 step 4): the two states a FAT delete or rename can tear into never
//! cost the only good copy.

use faraday_codec::vault_file::SIGNATURE;
use faraday_crypto::work::Compute;
use faraday_fat::{Disk, Region, Volume, format32};
use faraday_files::{
    Fat, OnTaken, Place, Wiped, finish_vault_writes, write_replacing, write_vault_over,
};
use faraday_text::Text;

/// A whole KDBX 4 file of a vault's shape (as `vault_by_contents.rs`).
fn vault(salt: u8) -> Vec<u8> {
    vault_of(salt, 65_536)
}

/// The same with a body of `body` bytes.
fn vault_of(salt: u8, body: usize) -> Vec<u8> {
    let mut v = SIGNATURE.to_vec();
    v.extend_from_slice(&[0, 0, 4, 0]);
    v.extend_from_slice(&[0, 0, 0, 0, 0]);
    v.extend_from_slice(&[salt; 64]);
    v.extend_from_slice(&[9; 32]);
    v.extend_from_slice(&(body as u32).to_le_bytes());
    v.resize(v.len() + body, 5);
    v.extend_from_slice(&[9; 32]);
    v.extend_from_slice(&0u32.to_le_bytes());
    v
}

const PART: &str = ".faraday-mine.kdbx.part";

/// A FAT32 volume in memory with the original and its part file.
fn stick(original: &[u8], part: &[u8]) -> Vec<u8> {
    stick_named("mine.kdbx", original, part)
}

fn stick_named(name: &str, original: &[u8], part: &[u8]) -> Vec<u8> {
    let sectors = 70_000u64;
    let mut img = vec![0u8; sectors as usize * 512];
    for r in format32(sectors, 0, 1, None).unwrap().regions() {
        if let Region::Bytes { at, data } = r {
            img[at as usize..at as usize + data.len()].copy_from_slice(&data);
        }
    }
    let mut place = Fat(Volume::open(img).unwrap());
    place.create(name, original).unwrap();
    place
        .create(&format!(".faraday-{name}.part"), part)
        .unwrap();
    place.0.flush().unwrap();
    place.0.into_disk()
}

fn u32_at(img: &[u8], at: usize) -> usize {
    u32::from_le_bytes(img[at..at + 4].try_into().unwrap()) as usize
}

/// Where the short entry of the `n`th file is in the root (in the order
/// they were made), and its first cluster.
fn entry_of(img: &[u8], n: usize) -> (usize, usize) {
    let mut seen = 0;
    let (spc, reserved) = (img[13] as usize, img[14] as usize | (img[15] as usize) << 8);
    let (nfats, fatsz, root) = (img[16] as usize, u32_at(img, 36), u32_at(img, 44));
    let at = (reserved + nfats * fatsz + (root - 2) * spc) * 512;
    for i in 0..spc * 512 / 32 {
        let e = at + i * 32;
        if img[e + 11] == 0x20 && img[e] != 0xE5 && img[e] != 0 {
            seen += 1;
            if seen != n + 1 {
                continue;
            }
            let hi = img[e + 20] as usize | (img[e + 21] as usize) << 8;
            let lo = img[e + 26] as usize | (img[e + 27] as usize) << 8;
            return (e, hi << 16 | lo);
        }
    }
    panic!("no entry");
}

#[test]
fn a_kept_entry_whose_clusters_were_freed_does_not_cost_the_whole_part_file() {
    let mut img = stick(&vault(1), &vault(2));
    // The torn delete: the chain is freed, the directory entry is not.
    let reserved = img[14] as usize | (img[15] as usize) << 8;
    let (_, mut c) = entry_of(&img, 0);
    while (2..0x0FFF_FFF8).contains(&c) {
        let at = reserved * 512 + c * 4;
        let next = u32_at(&img, at) & 0x0FFF_FFFF;
        img[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
        c = next;
    }
    let mut place = Fat(Volume::open(img).unwrap());
    finish_vault_writes(&mut place);
    // The broken original is gone and the part is the vault, under its
    // name.
    assert_eq!(place.read("mine.kdbx", u64::MAX).unwrap(), vault(2));
    assert!(place.read(PART, u64::MAX).is_err());
    let names: Vec<Text> = place.files().unwrap().into_iter().map(|f| f.0).collect();
    assert_eq!(names, ["mine.kdbx"]);
}

#[test]
fn an_original_whose_last_cluster_was_freed_keeps_the_part_file() {
    let mut img = stick(&vault(1), &vault(2));
    // A torn delete that reached only the FAT sector of the last
    // cluster: the chain still reads, but does not end.
    let reserved = img[14] as usize | (img[15] as usize) << 8;
    let (_, mut c) = entry_of(&img, 0);
    let mut last = c;
    while (2..0x0FFF_FFF8).contains(&c) {
        last = c;
        c = u32_at(&img, reserved * 512 + c * 4) & 0x0FFF_FFFF;
    }
    let at = reserved * 512 + last * 4;
    img[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
    let mut place = Fat(Volume::open(img).unwrap());
    finish_vault_writes(&mut place);
    assert_eq!(place.read("mine.kdbx", u64::MAX).unwrap(), vault(2));
}

#[test]
fn wiping_one_of_two_names_on_a_chain_leaves_the_other_readable() {
    let mut img = stick(&vault(1), &vault(2));
    let (part, first) = entry_of(&img, 1);
    let (orig, _) = entry_of(&img, 0);
    let src = img[part + 20..part + 22].to_vec();
    img[orig + 20..orig + 22].copy_from_slice(&src);
    img[orig + 26..orig + 28].copy_from_slice(&(first as u16).to_le_bytes());
    let mut place = Fat(Volume::open(img).unwrap());
    place.wipe(PART).unwrap();
    assert_eq!(place.read("mine.kdbx", u64::MAX).unwrap(), vault(2));
}

#[test]
fn a_vault_named_as_a_device_is_not_lost_to_a_cut_before_the_rename() {
    let img = stick_named("con.kdbx", &vault(1), &vault(2));
    let mut place = Fat(Volume::open(img).unwrap());
    // Cut after the original was removed, before the rename.
    place.remove("con.kdbx").unwrap();
    finish_vault_writes(&mut place);
    let names: Vec<Text> = place.files().unwrap().into_iter().map(|f| f.0).collect();
    assert_eq!(names, ["con.kdbx"]);
    assert_eq!(place.read("con.kdbx", u64::MAX).unwrap(), vault(2));
}

#[test]
fn two_names_on_one_chain_leave_the_vault_whole_after_another_file_is_written() {
    let mut img = stick(&vault(1), &vault(2));
    // The torn rename: the original's entry points at the part's chain.
    let (part, first) = entry_of(&img, 1);
    let (orig, _) = entry_of(&img, 0);
    let src = img[part + 20..part + 22].to_vec();
    img[orig + 20..orig + 22].copy_from_slice(&src);
    img[orig + 26..orig + 28].copy_from_slice(&(first as u16).to_le_bytes());
    let mut place = Fat(Volume::open(img).unwrap());
    finish_vault_writes(&mut place);
    place.create("other.bin", &[7u8; 70_000]).unwrap();
    assert_eq!(place.read("mine.kdbx", u64::MAX).unwrap(), vault(2));
}

/// The chain of the `n`th file's clusters, from the first copy of the FAT.
fn chain_of(img: &[u8], n: usize) -> Vec<usize> {
    let reserved = img[14] as usize | (img[15] as usize) << 8;
    let (_, mut c) = entry_of(img, n);
    let mut out = Vec::new();
    while (2..0x0FFF_FFF8).contains(&c) {
        out.push(c);
        c = u32_at(img, reserved * 512 + c * 4) & 0x0FFF_FFFF;
    }
    out
}

/// The original's chain runs on into the part file's clusters, as after
/// a torn delete freed its last cluster and a file took it.
fn cross_linked() -> Vec<u8> {
    let mut img = stick(&vault(1), &vault(2));
    let reserved = img[14] as usize | (img[15] as usize) << 8;
    let last = *chain_of(&img, 0).last().unwrap();
    let first = chain_of(&img, 1)[0];
    let at = reserved * 512 + last * 4;
    img[at..at + 4].copy_from_slice(&(first as u32).to_le_bytes());
    img
}

#[test]
fn discarding_an_original_whose_chain_runs_into_a_file_in_a_folder_leaves_that_file() {
    let mut img = cross_linked();
    let theirs = chain_of(&img, 1);
    // That file's entry is not in the root: it is in a folder.
    let (mut e, _) = entry_of(&img, 1);
    img[e] = 0xE5;
    while e >= 32 && img[e - 32 + 11] == 0x0F {
        e -= 32;
        img[e] = 0xE5;
    }
    let mut v = Volume::open(img).unwrap();
    v.discard("mine.kdbx").unwrap();
    v.flush().unwrap();
    let img = v.into_disk();
    let reserved = img[14] as usize | (img[15] as usize) << 8;
    for c in theirs {
        assert_ne!(u32_at(&img, reserved * 512 + c * 4), 0, "cluster {c} freed");
    }
}

#[test]
fn wiping_a_name_whose_chain_runs_through_another_file_leaves_that_file_whole() {
    let img = cross_linked();
    let mut place = Fat(Volume::open(img).unwrap());
    place.wipe("mine.kdbx").unwrap();
    assert_eq!(place.read(PART, u64::MAX).unwrap(), vault(2));
}

/// A stick whose medium misses the writes inside `lost`, while its cache
/// (what `read_at` answers) takes them all: a controller that drops
/// writes, or wraps them onto other blocks.
struct Lossy {
    cache: Vec<u8>,
    medium: Vec<u8>,
    lost: std::ops::Range<usize>,
}

impl Disk for Lossy {
    fn size(&self) -> u64 {
        self.cache.len() as u64
    }
    fn read_at(&mut self, at: u64, buf: &mut [u8]) -> Result<(), faraday_fat::Error> {
        self.cache.read_at(at, buf)
    }
    fn write_at(&mut self, at: u64, buf: &[u8]) -> Result<(), faraday_fat::Error> {
        self.cache.write_at(at, buf)?;
        let (s, e) = (at as usize, at as usize + buf.len());
        if !(self.lost.start <= s && e <= self.lost.end) {
            self.medium.write_at(at, buf)?;
        }
        Ok(())
    }
    fn read_back(&mut self, at: u64, buf: &mut [u8]) -> Result<(), faraday_fat::Error> {
        self.medium.read_at(at, buf)
    }
}

fn blank_stick(sectors: u64) -> Vec<u8> {
    let mut img = vec![0u8; sectors as usize * 512];
    for r in format32(sectors, 0, 1, None).unwrap().regions() {
        if let Region::Bytes { at, data } = r {
            img[at as usize..at as usize + data.len()].copy_from_slice(&data);
        }
    }
    img
}

/// Writes over `notes.txt` on a stick that loses the writes in the range
/// `lost` of the image; the range is found from the boot sector.
fn replace_losing(
    len: usize,
    lost: impl Fn(&[u8]) -> std::ops::Range<usize>,
) -> (Result<Text, Text>, Vec<u8>) {
    let mut img = blank_stick(70_000);
    let mut v = Volume::open(img.clone()).unwrap();
    v.create("notes.txt", b"old").unwrap();
    v.flush().unwrap();
    img = v.into_disk();
    let lost = lost(&img);
    let disk = Lossy {
        cache: img.clone(),
        medium: img,
        lost,
    };
    let mut place = Fat(Volume::open(disk).unwrap());
    let r = write_replacing(&mut place, "notes.txt", &vec![7u8; len]);
    let disk = place.0.into_disk();
    (r, disk.medium)
}

#[test]
fn a_stick_that_drops_the_fat_writes_fails_the_check_and_keeps_the_old_file() {
    let (r, medium) = replace_losing(5000, |img| {
        let reserved = img[14] as usize | (img[15] as usize) << 8;
        reserved * 512..(reserved + u32_at(img, 36) * img[16] as usize) * 512
    });
    assert!(r.is_err());
    let mut place = Fat(Volume::open(medium).unwrap());
    assert_eq!(place.read("notes.txt", u64::MAX).unwrap(), b"old");
}

#[test]
fn a_stick_that_drops_the_directory_writes_fails_the_check_and_keeps_the_old_file() {
    let (r, medium) = replace_losing(5000, |img| {
        let (spc, reserved) = (img[13] as usize, img[14] as usize | (img[15] as usize) << 8);
        let (nfats, fatsz, root) = (img[16] as usize, u32_at(img, 36), u32_at(img, 44));
        let at = (reserved + nfats * fatsz + (root - 2) * spc) * 512;
        at..at + spc * 512
    });
    assert!(r.is_err());
    let mut place = Fat(Volume::open(medium).unwrap());
    assert_eq!(place.read("notes.txt", u64::MAX).unwrap(), b"old");
}

#[test]
fn a_stick_that_drops_the_fat_write_of_a_one_cluster_vault_fails_the_check_and_keeps_the_old_file()
{
    let (r, medium) = replace_losing(300, |img| {
        let reserved = img[14] as usize | (img[15] as usize) << 8;
        reserved * 512..(reserved + u32_at(img, 36) * img[16] as usize) * 512
    });
    assert!(r.is_err());
    let mut place = Fat(Volume::open(medium).unwrap());
    assert_eq!(place.read("notes.txt", u64::MAX).unwrap(), b"old");
}

/// A stick with `MINE.KDBX` and a file `INNER.BIN` in the folder `SUB`,
/// each one cluster, the name `MINE.KDBX` on the very cluster of
/// `INNER.BIN`: a chain of the right length that the root does not show
/// another file on.
fn name_on_a_file_in_a_folder() -> Vec<u8> {
    let mut img = blank_stick(70_000);
    let mut v = Volume::open(img.clone()).unwrap();
    v.create("MINE.KDBX", &[1u8; 300]).unwrap();
    v.create("INNER.BIN", &[2u8; 100]).unwrap();
    v.create("SUB", &[0u8; 512]).unwrap();
    v.flush().unwrap();
    img = v.into_disk();
    let (spc, reserved) = (img[13] as usize, img[14] as usize | (img[15] as usize) << 8);
    let (nfats, fatsz, root) = (img[16] as usize, u32_at(&img, 36), u32_at(&img, 44));
    let data = (reserved + nfats * fatsz) * 512;
    let cluster_at = |c: usize| data + (c - 2) * spc * 512;
    let (mine, _) = entry_of(&img, 0);
    let (inner, x) = entry_of(&img, 1);
    let (sub, d) = entry_of(&img, 2);
    assert_eq!(root, 2);
    // INNER.BIN's entry moves into the folder's cluster.
    let moved = img[inner..inner + 32].to_vec();
    img[cluster_at(d)..cluster_at(d) + spc * 512].fill(0);
    img[cluster_at(d)..cluster_at(d) + 32].copy_from_slice(&moved);
    img[inner] = 0xE5;
    // SUB becomes a folder.
    img[sub + 11] = 0x10;
    img[sub + 28..sub + 32].fill(0);
    // MINE.KDBX starts at INNER.BIN's cluster.
    img[mine + 20..mine + 22].copy_from_slice(&((x >> 16) as u16).to_le_bytes());
    img[mine + 26..mine + 28].copy_from_slice(&(x as u16).to_le_bytes());
    img
}

#[test]
fn wiping_a_name_on_a_file_in_a_folder_leaves_that_file_whole() {
    let mut v = Volume::open(name_on_a_file_in_a_folder()).unwrap();
    v.wipe("MINE.KDBX").unwrap();
    assert_eq!(
        v.read_in(&["SUB", "INNER.BIN"], u32::MAX).unwrap(),
        [2u8; 100]
    );
}

#[test]
fn discarding_a_name_on_a_file_in_a_folder_leaves_that_file_whole() {
    let mut v = Volume::open(name_on_a_file_in_a_folder()).unwrap();
    v.discard("MINE.KDBX").unwrap();
    // Something written after it takes a free cluster, as the next
    // computer would.
    v.create("NEXT.BIN", &[9u8; 100]).unwrap();
    assert_eq!(
        v.read_in(&["SUB", "INNER.BIN"], u32::MAX).unwrap(),
        [2u8; 100]
    );
}

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-files' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

/// Sets the FAT entry of cluster `c` in every copy of the FAT.
fn set_fat(img: &mut [u8], c: usize, v: u32) {
    let reserved = img[14] as usize | (img[15] as usize) << 8;
    let (nfats, fatsz) = (img[16] as usize, u32_at(img, 36));
    for k in 0..nfats {
        let at = (reserved + k * fatsz) * 512 + c * 4;
        img[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
}

/// Where the clusters start and how many bytes one has.
fn data_of(img: &[u8]) -> (usize, usize) {
    let (spc, reserved) = (img[13] as usize, img[14] as usize | (img[15] as usize) << 8);
    let (nfats, fatsz) = (img[16] as usize, u32_at(img, 36));
    ((reserved + nfats * fatsz) * 512, spc * 512)
}

#[test]
fn a_wipe_that_cannot_walk_every_folder_removes_only_the_name_and_leaves_the_bytes() {
    let mut img = blank_stick(70_000);
    let mut v = Volume::open(img).unwrap();
    v.create("MINE.KDBX", &[0x5Au8; 300]).unwrap();
    v.create("OTHER.BIN", &[2u8; 100]).unwrap();
    v.flush().unwrap();
    img = v.into_disk();
    // OTHER.BIN starts at a cluster outside the volume: what follows
    // from it cannot be told.
    let (other, _) = entry_of(&img, 1);
    img[other + 20..other + 22].copy_from_slice(&0x0FFFu16.to_le_bytes());
    img[other + 26..other + 28].copy_from_slice(&0xFFF0u16.to_le_bytes());
    let mut v = Volume::open(img).unwrap();
    assert_eq!(v.wipe("MINE.KDBX").unwrap(), Wiped::NameOnly);
    let img = v.into_disk();
    assert!(img.windows(300).any(|w| w == [0x5Au8; 300]));
}

#[test]
fn a_wipe_of_a_file_alone_on_its_clusters_zeros_them() {
    let mut img = blank_stick(70_000);
    let mut v = Volume::open(std::mem::take(&mut img)).unwrap();
    v.create("MINE.KDBX", &[0x5Au8; 300]).unwrap();
    assert_eq!(v.wipe("MINE.KDBX").unwrap(), Wiped::Zeroed);
    assert!(!v.into_disk().windows(300).any(|w| w == [0x5Au8; 300]));
}

#[test]
fn a_wipe_with_a_folder_that_loops_or_runs_longer_than_a_root_ends_with_only_the_name_removed() {
    for long in [false, true] {
        let mut img = blank_stick(70_000);
        let mut v = Volume::open(img).unwrap();
        v.create("MINE.KDBX", &[0x5Au8; 300]).unwrap();
        v.create("SUB", &[0u8; 512]).unwrap();
        v.flush().unwrap();
        img = v.into_disk();
        let (data, cb) = data_of(&img);
        let (sub, d) = entry_of(&img, 1);
        img[sub + 11] = 0x10;
        img[sub + 28..sub + 32].fill(0);
        let at = data + (d - 2) * cb;
        img[at..at + cb].fill(0);
        if long {
            // A chain of more clusters than a root directory may have,
            // through clusters nothing else holds.
            let n = 2 * 1024 * 1024 / cb + 1;
            for k in 0..n {
                let next = if k + 1 == n {
                    0x0FFF_FFFF
                } else {
                    (d + 1000 + k + 1) as u32
                };
                set_fat(&mut img, if k == 0 { d } else { d + 1000 + k }, next);
            }
            set_fat(&mut img, d, (d + 1001) as u32);
        } else {
            // A folder whose chain runs back into itself, its cluster
            // holding a thousand entries of itself.
            set_fat(&mut img, d, d as u32);
            let own = img[sub..sub + 32].to_vec();
            for i in 0..cb / 32 {
                img[at + i * 32..at + i * 32 + 32].copy_from_slice(&own);
            }
        }
        let mut v = Volume::open(img).unwrap();
        assert_eq!(
            v.wipe("MINE.KDBX").unwrap(),
            Wiped::NameOnly,
            "long: {long}"
        );
        assert!(v.into_disk().windows(300).any(|w| w == [0x5Au8; 300]));
    }
}

#[test]
fn a_vault_write_over_a_name_whose_chain_runs_into_another_file_leaves_that_file_whole() {
    let old = vault_of(1, 256);
    let mut img = blank_stick(70_000);
    let mut v = Volume::open(img).unwrap();
    v.create("MINE.KDBX", &old).unwrap();
    v.create("INNER.BIN", &[2u8; 100]).unwrap();
    v.create("SUB", &[0u8; 512]).unwrap();
    v.flush().unwrap();
    img = v.into_disk();
    let (data, cb) = data_of(&img);
    assert!(old.len() <= cb);
    let (inner, x) = entry_of(&img, 1);
    let (mine, m) = entry_of(&img, 0);
    let (sub, d) = entry_of(&img, 2);
    // INNER.BIN's entry moves into the folder SUB.
    let moved = img[inner..inner + 32].to_vec();
    img[data + (d - 2) * cb..data + (d - 1) * cb].fill(0);
    img[data + (d - 2) * cb..data + (d - 2) * cb + 32].copy_from_slice(&moved);
    img[inner] = 0xE5;
    img[sub + 11] = 0x10;
    img[sub + 28..sub + 32].fill(0);
    // The vault's one cluster is followed in the FAT by INNER.BIN's.
    let _ = mine;
    set_fat(&mut img, m, x as u32);
    let mut place = Fat(Volume::open(img).unwrap());
    let new = vault_of(1, 250);
    let copy = faraday_crypto::sha256(&old);
    write_vault_over(
        &compute(),
        &mut place,
        "MINE.KDBX",
        &new,
        &[copy],
        true,
        false,
        OnTaken::Ask,
    )
    .unwrap();
    assert_eq!(place.read("MINE.KDBX", u64::MAX).unwrap(), new);
    // What is written next takes the clusters the write freed.
    place.create("NEXT.BIN", &vec![9u8; cb * 3]).unwrap();
    assert_eq!(
        place.0.read_in(&["SUB", "INNER.BIN"], u32::MAX).unwrap(),
        [2u8; 100]
    );
}

/// A write-back over an exact-length one-cluster original whose cluster is
/// also another file's (`inner`: a file in a folder, or the second cluster
/// of the root) leaves that file whole.
fn write_back_over_shared(first: bool) {
    let old = vault_of(1, 256);
    let mut img = blank_stick(70_000);
    let mut v = Volume::open(img).unwrap();
    v.create("MINE.KDBX", &old).unwrap();
    v.create("INNER.BIN", &[2u8; 100]).unwrap();
    v.create("SUB", &[0u8; 512]).unwrap();
    v.flush().unwrap();
    img = v.into_disk();
    let (data, cb) = data_of(&img);
    let (inner, x) = entry_of(&img, 1);
    let (mine, _) = entry_of(&img, 0);
    let (sub, d) = entry_of(&img, 2);
    let target = if first {
        // INNER.BIN's entry moves into the folder SUB.
        let moved = img[inner..inner + 32].to_vec();
        img[data + (d - 2) * cb..data + (d - 1) * cb].fill(0);
        img[data + (d - 2) * cb..data + (d - 2) * cb + 32].copy_from_slice(&moved);
        img[inner] = 0xE5;
        img[sub + 11] = 0x10;
        img[sub + 28..sub + 32].fill(0);
        // Both files' bytes are what is in the cluster they share.
        img[data + (x - 2) * cb..data + (x - 2) * cb + old.len()].copy_from_slice(&old);
        x
    } else {
        // The root's chain gets a second cluster, MINE.KDBX's own.
        let (_, m) = entry_of(&img, 0);
        set_fat(&mut img, 2, m as u32);
        set_fat(&mut img, m, 0x0FFF_FFFF);
        m
    };
    // The vault's one cluster is `target`.
    img[mine + 20..mine + 22].copy_from_slice(&((target >> 16) as u16).to_le_bytes());
    img[mine + 26..mine + 28].copy_from_slice(&(target as u16).to_le_bytes());
    let mut place = Fat(Volume::open(img).unwrap());
    let new = vault_of(1, 250);
    let copy = faraday_crypto::sha256(&old);
    let done = write_vault_over(
        &compute(),
        &mut place,
        "MINE.KDBX",
        &new,
        &[copy],
        true,
        false,
        OnTaken::Ask,
    );
    done.unwrap();
    assert_eq!(place.read("MINE.KDBX", u64::MAX).unwrap(), new);
    place.create("NEXT.BIN", &vec![9u8; cb * 3]).unwrap();
    if first {
        assert_eq!(
            place.0.read_in(&["SUB", "INNER.BIN"], u32::MAX).unwrap(),
            old[..100]
        );
    } else {
        // The root's second cluster is as it was, and the root lists.
        let img = place.0.into_disk();
        let at = data + (target - 2) * cb;
        assert_eq!(img[at..at + old.len()], old[..]);
        let mut v = Volume::open(img).unwrap();
        assert!(v.list().unwrap().iter().any(|e| e.name == "INNER.BIN"));
    }
}

#[test]
fn a_write_back_over_an_original_on_a_folders_file_cluster_leaves_that_file_whole() {
    write_back_over_shared(true);
}

#[test]
fn a_write_back_over_an_original_on_the_roots_second_cluster_leaves_the_root_whole() {
    write_back_over_shared(false);
}

#[test]
fn a_wipe_of_a_file_whose_chain_is_far_longer_than_its_size_and_loops_removes_only_the_name() {
    let mut img = blank_stick(70_000);
    let mut v = Volume::open(img).unwrap();
    v.create("MINE.KDBX", &[0x5Au8; 300]).unwrap();
    v.flush().unwrap();
    img = v.into_disk();
    let (_, m) = entry_of(&img, 0);
    // A chain of 5,000 clusters for 300 bytes.
    for k in 0..5000 {
        let c = if k == 0 { m } else { m + 100 + k };
        // The end of it runs back to its start.
        let next = if k == 4999 {
            m as u32
        } else {
            (m + 101 + k) as u32
        };
        set_fat(&mut img, c, next);
    }
    let mut v = Volume::open(img).unwrap();
    assert_eq!(v.wipe("MINE.KDBX").unwrap(), Wiped::NameOnly);
    assert!(v.into_disk().windows(300).any(|w| w == [0x5Au8; 300]));
}

/// A disk that counts its reads.
struct Counting(Vec<u8>, std::rc::Rc<std::cell::Cell<usize>>);

impl Disk for Counting {
    fn size(&self) -> u64 {
        self.0.size()
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), faraday_fat::Error> {
        self.1.set(self.1.get() + 1);
        self.0.read_at(offset, buf)
    }
    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<(), faraday_fat::Error> {
        self.0.write_at(offset, buf)
    }
}

#[test]
fn a_visit_beside_a_part_does_not_walk_an_original_chain_far_past_its_size() {
    let mut img = stick(&vault(1), &vault(2));
    let last = *chain_of(&img, 0).last().unwrap();
    let top = chain_of(&img, 1).into_iter().max().unwrap().max(last);
    // The original's chain runs on through 20,000 more clusters.
    let run = 20_000;
    set_fat(&mut img, last, (top + 100) as u32);
    for k in 0..run {
        let c = top + 100 + k;
        let next = if k + 1 == run {
            0x0FFF_FFFF
        } else {
            (c + 1) as u32
        };
        set_fat(&mut img, c, next);
    }
    let reads = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut place = Fat(Volume::open(Counting(img, reads.clone())).unwrap());
    let before = reads.get();
    finish_vault_writes(&mut place);
    // The part is the vault under its name, and the walk was not made to
    // the end of the chain.
    assert_eq!(place.read("mine.kdbx", u64::MAX).unwrap(), vault(2));
    assert!(
        reads.get() - before < run / 2,
        "{} reads",
        reads.get() - before
    );
}

/// A part file planted on a stick whose header fields' sizes reach the
/// end of the address space: the visit that finishes vault writes comes
/// back (it neither hangs nor stops the process) and the vault beside it
/// is left as it was.
#[test]
fn a_part_file_with_header_sizes_that_wrap_does_not_hang_the_visit() {
    let mut part = SIGNATURE.to_vec();
    part.extend_from_slice(&[0, 0, 4, 0]);
    part.push(0);
    part.extend_from_slice(&0xFFFF_FFA0u32.to_le_bytes());
    part.extend_from_slice(&0xFFFF_FFDCu32.to_le_bytes());
    part.resize(400, 0);
    let img = stick(&vault(1), &part);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut place = Fat(Volume::open(img).unwrap());
        finish_vault_writes(&mut place);
        let _ = tx.send(place.read("mine.kdbx", u64::MAX).unwrap());
    });
    let kept = rx
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect("the visit did not come back within 30 s");
    assert_eq!(kept, vault(1));
}

/// A KDBX 4 file with a header as KeePass writes one: the cipher,
/// compression, master seed, IV and KDF parameters, and the end field
/// with its four bytes, then the header's hash and HMAC and a stream of
/// two blocks and the empty block.
fn kdbx4_as_written() -> Vec<u8> {
    let mut v = SIGNATURE.to_vec();
    v.extend_from_slice(&[1, 0, 4, 0]);
    let mut field = |id: u8, data: &[u8]| {
        v.push(id);
        v.extend_from_slice(&(data.len() as u32).to_le_bytes());
        v.extend_from_slice(data);
    };
    field(2, &[0x31; 16]);
    field(3, &1u32.to_le_bytes());
    field(4, &[0x42; 32]);
    field(7, &[0x17; 12]);
    field(11, &[0x5A; 141]);
    field(0, b"\r\n\r\n");
    v.extend_from_slice(&[0x66; 64]);
    for body in [1_048_576usize, 3_000, 0] {
        v.extend_from_slice(&[0x77; 32]);
        v.extend_from_slice(&(body as u32).to_le_bytes());
        v.resize(v.len() + body, 0x88);
    }
    v
}

/// The part file of such a vault whose original is gone takes the
/// vault's name; the same file cut short is not whole, and goes.
#[test]
fn a_part_file_with_a_header_as_keepass_writes_it_takes_the_name_only_when_whole() {
    let whole = kdbx4_as_written();
    for (part, kept) in [
        (whole.clone(), true),
        (whole[..whole.len() - 1].to_vec(), false),
    ] {
        let mut place = Fat(Volume::open(stick(&vault(1), &part)).unwrap());
        place.remove("mine.kdbx").unwrap();
        finish_vault_writes(&mut place);
        let names: Vec<Text> = place.files().unwrap().into_iter().map(|f| f.0).collect();
        if kept {
            assert_eq!(names, ["mine.kdbx"]);
            assert_eq!(place.read("mine.kdbx", u64::MAX).unwrap(), whole);
        } else {
            assert!(names.is_empty(), "{names:?}");
        }
    }
}
