//! The test EFI image: a minimal PE32+ EFI application, for Secure Boot signing: headers and one
//! section holding a return. It does nothing if started.

#[must_use]
pub(crate) fn image() -> Vec<u8> {
    let mut pe = vec![0u8; 0x400];
    let put16 =
        |pe: &mut Vec<u8>, at: usize, v: u16| pe[at..at + 2].copy_from_slice(&v.to_le_bytes());
    let put32 =
        |pe: &mut Vec<u8>, at: usize, v: u32| pe[at..at + 4].copy_from_slice(&v.to_le_bytes());
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
    put32(&mut pe, opt + 56, 0x2000);
    put32(&mut pe, opt + 60, 0x200);
    put16(&mut pe, opt + 68, 10);
    put32(&mut pe, opt + 108, 16);
    let sec = opt + 240;
    pe[sec..sec + 5].copy_from_slice(b".text");
    put32(&mut pe, sec + 8, 0x10);
    put32(&mut pe, sec + 12, 0x1000);
    put32(&mut pe, sec + 16, 0x200);
    put32(&mut pe, sec + 20, 0x200);
    put32(&mut pe, sec + 36, 0x6000_0020);
    pe[0x200] = 0xc3;
    pe
}
