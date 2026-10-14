//! A Start menu shortcut's bytes (`docs/STARTER.md` §5.2): a Shell Link
//! (`.lnk`) file as Microsoft's [MS-SHLLINK] lays it out, written
//! directly rather than through the shell's COM object, so it is plain
//! data the tests can read back and needs no `unsafe`.
//!
//! It has a header, a `LinkInfo` naming the target by its local path
//! (ANSI and Unicode), the working folder and the icon's place as
//! Unicode strings, and the terminal block. No item ID list: the shell
//! resolves a link from its `LinkInfo` when there is none.

/// `LinkFlags`.
const HAS_LINK_INFO: u32 = 0x02;
const HAS_WORKING_DIR: u32 = 0x10;
const HAS_ICON_LOCATION: u32 = 0x40;
const IS_UNICODE: u32 = 0x80;
/// `FILE_ATTRIBUTE_ARCHIVE`.
const ARCHIVE: u32 = 0x20;
/// `SW_SHOWNORMAL`.
const SHOW_NORMAL: u32 = 1;
/// `DRIVE_FIXED`.
const DRIVE_FIXED: u32 = 3;
/// The Shell Link class, `00021401-0000-0000-C000-000000000046`.
const CLSID: [u8; 16] = [
    0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
];
/// The header's size, and `LinkInfo`'s header with its Unicode offsets.
pub const HEADER: usize = 0x4c;
const LINK_INFO_HEADER: u32 = 0x24;

fn utf16z(s: &str) -> Vec<u8> {
    s.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// `s` in the ANSI code page as far as it is ASCII; other characters
/// as `?` (the Unicode copy beside it is what the shell reads).
fn ansiz(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
        .chain([0])
        .collect()
}

/// A `StringData` entry: its length in UTF-16 units, then the units.
fn string_data(s: &str) -> Vec<u8> {
    let units: Vec<u16> = s.encode_utf16().collect();
    let mut out = (units.len() as u16).to_le_bytes().to_vec();
    out.extend(units.iter().flat_map(|u| u.to_le_bytes()));
    out
}

/// A shortcut to `target` (a full path), started in `folder`, with the
/// icon in `target` itself.
pub fn lnk(target: &str, folder: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend((HEADER as u32).to_le_bytes());
    out.extend(CLSID);
    out.extend((HAS_LINK_INFO | HAS_WORKING_DIR | HAS_ICON_LOCATION | IS_UNICODE).to_le_bytes());
    out.extend(ARCHIVE.to_le_bytes());
    out.extend([0u8; 24]); // creation, access and write times
    out.extend(0u32.to_le_bytes()); // file size
    out.extend(0i32.to_le_bytes()); // icon index
    out.extend(SHOW_NORMAL.to_le_bytes());
    out.extend([0u8; 2 + 2 + 4 + 4]); // hot key, reserved
    out.extend(link_info(target));
    out.extend(string_data(folder));
    out.extend(string_data(target));
    out.extend(0u32.to_le_bytes()); // the terminal block
    out
}

/// `LinkInfo` with `VolumeIDAndLocalBasePath`: a volume ID for a fixed
/// drive with no label, the path in ANSI, an empty suffix, then both
/// again in Unicode.
fn link_info(target: &str) -> Vec<u8> {
    let mut volume = Vec::new();
    volume.extend(17u32.to_le_bytes()); // its size: 16 and the label's NUL
    volume.extend(DRIVE_FIXED.to_le_bytes());
    volume.extend(0u32.to_le_bytes()); // serial number
    volume.extend(0x10u32.to_le_bytes()); // label offset
    volume.push(0);
    let base = ansiz(target);
    let suffix = vec![0u8];
    let base_w = utf16z(target);
    let suffix_w = utf16z("");
    let volume_at = LINK_INFO_HEADER;
    let base_at = volume_at + volume.len() as u32;
    let suffix_at = base_at + base.len() as u32;
    let base_w_at = suffix_at + suffix.len() as u32;
    let suffix_w_at = base_w_at + base_w.len() as u32;
    let size = suffix_w_at + suffix_w.len() as u32;
    let mut out = Vec::new();
    for v in [
        size,
        LINK_INFO_HEADER,
        1, // VolumeIDAndLocalBasePath
        volume_at,
        base_at,
        0, // no network path
        suffix_at,
        base_w_at,
        suffix_w_at,
    ] {
        out.extend(v.to_le_bytes());
    }
    out.extend(volume);
    out.extend(base);
    out.extend(suffix);
    out.extend(base_w);
    out.extend(suffix_w);
    out
}

/// The target a shortcut made by [`lnk`] names (its `LinkInfo`'s
/// Unicode path), or `None` when the bytes are not such a link.
pub fn target(b: &[u8]) -> Option<String> {
    let u32_at = |at: usize| Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?));
    if u32_at(0)? as usize != HEADER || b.get(4..20)? != CLSID {
        return None;
    }
    let flags = u32_at(20)?;
    if flags & HAS_LINK_INFO == 0 || flags & 0x01 != 0 {
        return None;
    }
    let info = HEADER;
    let size = u32_at(info)? as usize;
    if u32_at(info + 4)? < LINK_INFO_HEADER {
        return None;
    }
    let at = info + u32_at(info + 28)? as usize;
    let end = info + size;
    let units: Vec<u16> = b
        .get(at..end)?
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16(&units).ok()
}
