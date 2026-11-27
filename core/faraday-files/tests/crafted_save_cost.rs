//! A stick crafted with hundreds of files of a vault's length cannot make
//! a save read and hash them all: the save is refused after the listing.
//! Counted in bytes read from the medium, which does not change with the
//! machine (a stated cost says how it was measured).

use std::cell::Cell;
use std::rc::Rc;

use faraday_codec::vault_file::SIGNATURE;
use faraday_crypto::work::Compute;
use faraday_fat::{Disk, Error, Region, Volume, format32};
use faraday_files::{Fat, Place, write_vault};

fn vault(salt: u8) -> Vec<u8> {
    let body = 4096usize;
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

struct Counted {
    bytes: Vec<u8>,
    read: Rc<Cell<usize>>,
}

impl Disk for Counted {
    fn size(&self) -> u64 {
        self.bytes.len() as u64
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), Error> {
        self.read.set(self.read.get() + buf.len());
        self.bytes.read_at(offset, buf)
    }
    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<(), Error> {
        self.bytes.write_at(offset, buf)
    }
}

fn stick(files: usize) -> Vec<u8> {
    stick_of(files, vault)
}

fn stick_of(files: usize, body: fn(u8) -> Vec<u8>) -> Vec<u8> {
    let sectors = 70_000u64;
    let mut img = vec![0u8; sectors as usize * 512];
    for r in format32(sectors, 0, 1, None).unwrap().regions() {
        if let Region::Bytes { at, data } = r {
            img[at as usize..at as usize + data.len()].copy_from_slice(&data);
        }
    }
    let mut place = Fat(Volume::open(img).unwrap());
    for i in 0..files {
        place.create(&format!("f{i}.bin"), &body(i as u8)).unwrap();
    }
    place.0.flush().unwrap();
    place.0.into_disk()
}

#[test]
fn a_stick_with_hundreds_of_files_of_a_vaults_length_refuses_a_save_after_reading_little() {
    let read = Rc::new(Cell::new(0));
    let mut place = Fat(Volume::open(Counted {
        bytes: stick(300),
        read: read.clone(),
    })
    .unwrap());
    let c = compute();
    let before = read.get();
    let r = write_vault(&c, &mut place, "mine.kdbx", &vault(250), &[[1; 32]]);
    assert!(r.is_err());
    // 300 files of 4 KiB are 1.2 MB: a refusal reads the directory and
    // the head of each file, never the files whole.
    let used = read.get() - before;
    assert!(used < 400_000, "{used} bytes read");
}

#[test]
fn a_stick_with_a_hundred_other_files_of_a_vaults_size_still_takes_a_save() {
    let mut place = Fat(Volume::open(stick_of(100, |i| vec![i; 4200])).unwrap());
    let c = compute();
    let r = write_vault(&c, &mut place, "mine.kdbx", &vault(250), &[[1; 32]]);
    assert!(r.is_ok(), "{r:?}");
}

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-files' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(faraday_crypto::work::Hooks {
        count: |_| {},
        stop: || false,
        progress: |_, _| {},
    })
}
