//! A crafted stick's filesystem: a chain that loops is refused after a
//! few reads, and a name with characters that hide is shown without them.

use std::cell::Cell;
use std::rc::Rc;

use faraday_fat::{Disk, Error, Fault, Region, Volume, format32};

/// A FAT32 volume in memory.
fn image() -> Vec<u8> {
    let sectors = 70_000u64;
    let mut img = vec![0u8; sectors as usize * 512];
    for r in format32(sectors, 0, 1, None).unwrap().regions() {
        if let Region::Bytes { at, data } = r {
            img[at as usize..at as usize + data.len()].copy_from_slice(&data);
        }
    }
    img
}

/// A disk that counts its reads.
struct Reads {
    bytes: Vec<u8>,
    reads: Rc<Cell<usize>>,
}

impl Disk for Reads {
    fn size(&self) -> u64 {
        self.bytes.len() as u64
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), Error> {
        self.reads.set(self.reads.get() + 1);
        self.bytes.read_at(offset, buf)
    }
    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<(), Error> {
        self.bytes.write_at(offset, buf)
    }
}

#[test]
fn a_file_whose_chain_loops_is_refused_after_a_few_reads_however_long_its_size_claims() {
    let mut v = Volume::open(image()).unwrap();
    v.create("loop.bin", &[7u8; 1500]).unwrap();
    v.flush().unwrap();
    let mut img = v.into_disk();
    let reserved = usize::from(u16::from_le_bytes([img[14], img[15]]));
    let at = img.windows(11).position(|w| w == b"LOOP    BIN").unwrap();
    let first = u32::from(u16::from_le_bytes([img[at + 26], img[at + 27]]))
        | u32::from(u16::from_le_bytes([img[at + 20], img[at + 21]])) << 16;
    let fat = |c: u32| reserved * 512 + c as usize * 4;
    let mut last = first;
    loop {
        let next = u32::from_le_bytes(img[fat(last)..fat(last) + 4].try_into().unwrap());
        if next >= 0x0FFF_FFF8 {
            break;
        }
        last = next;
    }
    // The last cluster leads back to the first, and the size claims
    // 20 000 clusters.
    img[fat(last)..fat(last) + 4].copy_from_slice(&first.to_le_bytes());
    let claim = 20_000u32 * 512;
    img[at + 28..at + 32].copy_from_slice(&claim.to_le_bytes());
    let reads = Rc::new(Cell::new(0));
    let mut v = Volume::open(Reads {
        bytes: img,
        reads: reads.clone(),
    })
    .unwrap();
    let before = reads.get();
    let r = v.read("loop.bin", u32::MAX);
    assert!(matches!(r, Err(Error::Corrupt(Fault::Loop))), "{r:?}");
    assert!(reads.get() - before < 100, "{} reads", reads.get() - before);
}

#[test]
fn a_name_with_tag_characters_selectors_or_fillers_is_shown_with_them_replaced() {
    for hidden in [
        '\u{E0041}',
        '\u{FE0F}',
        '\u{E0100}',
        '\u{115F}',
        '\u{1160}',
        '\u{3164}',
        '\u{FFA0}',
    ] {
        let mut v = Volume::open(image()).unwrap();
        let name = format!("a{hidden}b.txt");
        v.create(&name, b"x").unwrap();
        let shown: Vec<String> = v
            .list()
            .unwrap()
            .into_iter()
            .map(|e| e.name.to_string())
            .collect();
        assert_eq!(shown, ["a\u{FFFD}b.txt"], "U+{:X}", hidden as u32);
    }
}
