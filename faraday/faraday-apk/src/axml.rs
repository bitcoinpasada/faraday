//! The little of Android's binary XML an APK's manifest is read for
//! (`docs/APK-SIGNING.md` §5): the string pool, the resource map, the
//! `manifest` element's `package`, `versionCode`, `versionCodeMajor` and
//! `versionName`, and `uses-sdk`'s `minSdkVersion`.
//!
//! One reading only: attributes are known by resource ID, as Android
//! knows them, never by the text of their names; anything that two
//! readers could take two ways is refused, and so is a value Faraday
//! would have to guess (a reference, a codename).

use alloc::string::String;
use alloc::vec::Vec;

/// `android:versionCode`.
const VERSION_CODE: u32 = 0x0101_021b;
/// `android:versionCodeMajor`.
const VERSION_CODE_MAJOR: u32 = 0x0101_0576;
/// `android:versionName`.
const VERSION_NAME: u32 = 0x0101_021c;
/// `android:minSdkVersion`.
const MIN_SDK: u32 = 0x0101_020c;

/// The most attributes the `manifest` and `uses-sdk` elements may carry.
const MAX_ATTRIBUTES: usize = 256;

const XML: u16 = 0x0003;
const STRING_POOL: u16 = 0x0001;
const RESOURCE_MAP: u16 = 0x0180;
const START_ELEMENT: u16 = 0x0102;
const END_ELEMENT: u16 = 0x0103;
const START_NS: u16 = 0x0100;
const END_NS: u16 = 0x0101;
const CDATA: u16 = 0x0104;

const TYPE_STRING: u8 = 0x03;
const TYPE_INT_DEC: u8 = 0x10;
const TYPE_INT_HEX: u8 = 0x11;

/// What the manifest states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// The `package` attribute.
    pub package: String,
    /// `versionCode`, 0 when absent.
    pub version_code: u32,
    /// `versionCodeMajor`, 0 when absent.
    pub version_code_major: u32,
    /// `versionName`, when there is one.
    pub version_name: Option<String>,
    /// `minSdkVersion`; 1 when absent, as Android reads it.
    pub min_sdk: u32,
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn i32_at(b: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// A string pool: the chunk's bytes, where its strings are.
struct Pool<'a> {
    chunk: &'a [u8],
    count: usize,
    utf8: bool,
    strings_at: usize,
}

impl<'a> Pool<'a> {
    fn read(chunk: &'a [u8]) -> Option<Pool<'a>> {
        if u16_at(chunk, 2)? != 0x1c {
            return None;
        }
        let count = u32_at(chunk, 8)? as usize;
        let styles = u32_at(chunk, 12)? as usize;
        let flags = u32_at(chunk, 16)?;
        let strings_at = u32_at(chunk, 20)? as usize;
        if count > 1 << 20 || styles > 1 << 20 {
            return None;
        }
        // The offsets of the strings, then of the styles, are inside.
        if 0x1c + (count + styles) * 4 > chunk.len() || strings_at > chunk.len() {
            return None;
        }
        Some(Pool {
            chunk,
            count,
            utf8: flags & 0x100 != 0,
            strings_at,
        })
    }

    fn get(&self, idx: usize) -> Option<String> {
        if idx >= self.count {
            return None;
        }
        let off = u32_at(self.chunk, 0x1c + idx * 4)? as usize;
        let at = self.strings_at.checked_add(off)?;
        if self.utf8 {
            let c = self.chunk;
            // The length in characters, then the length in bytes: each
            // one byte, or two when the first has its top bit set.
            let mut p = at;
            let mut len = || -> Option<usize> {
                let b = *c.get(p)? as usize;
                if b & 0x80 != 0 {
                    let n = ((b & 0x7f) << 8) | *c.get(p + 1)? as usize;
                    p += 2;
                    Some(n)
                } else {
                    p += 1;
                    Some(b)
                }
            };
            let _chars = len()?;
            let bytes = len()?;
            let s = c.get(p..p.checked_add(bytes)?)?;
            if *c.get(p + bytes)? != 0 {
                return None;
            }
            core::str::from_utf8(s).ok().map(String::from)
        } else {
            let c = self.chunk;
            let first = u16_at(c, at)? as usize;
            let (len, p) = if first & 0x8000 != 0 {
                ((first & 0x7fff) << 16 | u16_at(c, at + 2)? as usize, at + 4)
            } else {
                (first, at + 2)
            };
            let raw = c.get(p..p.checked_add(len.checked_mul(2)?)?)?;
            if u16_at(c, p + len * 2)? != 0 {
                return None;
            }
            let units = raw
                .chunks_exact(2)
                .map(|u| u16::from_le_bytes([u[0], u[1]]));
            char::decode_utf16(units)
                .collect::<Result<String, _>>()
                .ok()
        }
    }
}

/// One attribute of a start element.
struct Attr {
    ns: i32,
    name: i32,
    raw: i32,
    kind: u8,
    data: u32,
}

/// The manifest's facts, or `None` when it does not read in one way.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub fn parse(data: &[u8]) -> Option<Manifest> {
    if u16_at(data, 0)? != XML || u16_at(data, 2)? != 8 || u32_at(data, 4)? as usize != data.len() {
        return None;
    }
    let mut at = 8;
    let mut pool: Option<Pool> = None;
    let mut map: Option<Vec<u32>> = None;
    let mut depth = 0u32;
    let mut root_seen = false;
    let mut sdk_seen = false;
    let mut out = Manifest {
        package: String::new(),
        version_code: 0,
        version_code_major: 0,
        version_name: None,
        min_sdk: 1,
    };
    let mut package_seen = false;
    while at < data.len() {
        let kind = u16_at(data, at)?;
        let head = u16_at(data, at + 2)? as usize;
        let size = u32_at(data, at + 4)? as usize;
        if size < 8 || head < 8 || head > size || !size.is_multiple_of(4) {
            return None;
        }
        let chunk = data.get(at..at.checked_add(size)?)?;
        match kind {
            STRING_POOL => {
                if pool.is_some() || root_seen {
                    return None;
                }
                pool = Some(Pool::read(chunk)?);
            }
            RESOURCE_MAP => {
                if map.is_some() || pool.is_none() || root_seen || head != 8 {
                    return None;
                }
                let n = (size - 8) / 4;
                map = Some(
                    (0..n)
                        .map(|i| u32_at(chunk, 8 + i * 4))
                        .collect::<Option<Vec<u32>>>()?,
                );
            }
            START_NS | END_NS | CDATA => {
                pool.as_ref()?;
            }
            START_ELEMENT => {
                let pool = pool.as_ref()?;
                if head != 16
                    || u16_at(chunk, 24)? != 0x14
                    || u16_at(chunk, 26)? != 0x14
                    || size != 36 + u16_at(chunk, 28)? as usize * 20
                {
                    return None;
                }
                let ns = i32_at(chunk, 16)?;
                let name = i32_at(chunk, 20)?;
                let count = u16_at(chunk, 28)? as usize;
                depth += 1;
                // Elements of the first two levels have no namespace, and
                // an element has a bounded number of attributes.
                if (depth <= 2 && ns != -1) || count > MAX_ATTRIBUTES {
                    return None;
                }
                let element = if ns == -1 {
                    pool.get(usize::try_from(name).ok()?)?
                } else {
                    String::new()
                };
                let which = match (depth, element.as_str()) {
                    (1, "manifest") if !root_seen => {
                        root_seen = true;
                        1
                    }
                    (1, _) => return None,
                    (2, "uses-sdk") => {
                        if sdk_seen {
                            return None;
                        }
                        sdk_seen = true;
                        2
                    }
                    _ => 0,
                };
                if which != 0 {
                    let mut attrs = Vec::new();
                    for i in 0..count {
                        let a = 36 + i * 20;
                        attrs.push(Attr {
                            ns: i32_at(chunk, a)?,
                            name: i32_at(chunk, a + 4)?,
                            raw: i32_at(chunk, a + 8)?,
                            kind: *chunk.get(a + 15)?,
                            data: u32_at(chunk, a + 16)?,
                        });
                    }
                    if which == 1 {
                        manifest_attrs(&attrs, pool, map.as_deref(), &mut out, &mut package_seen)?;
                    } else {
                        sdk_attrs(&attrs, pool, map.as_deref(), &mut out)?;
                    }
                }
            }
            END_ELEMENT => {
                pool.as_ref()?;
                depth = depth.checked_sub(1)?;
            }
            _ => return None,
        }
        at += size;
    }
    if depth != 0 || !root_seen || !package_seen {
        return None;
    }
    Some(out)
}

/// The resource ID the map gives an attribute's name; 0 for none.
fn id_of(map: Option<&[u32]>, name: i32) -> u32 {
    usize::try_from(name)
        .ok()
        .and_then(|n| map?.get(n).copied())
        .unwrap_or(0)
}

/// No two attributes of an element are the same attribute: by name and
/// namespace, or by resource ID.
fn distinct(attrs: &[Attr], map: Option<&[u32]>) -> bool {
    for (i, a) in attrs.iter().enumerate() {
        for b in &attrs[i + 1..] {
            if a.ns == b.ns && a.name == b.name {
                return false;
            }
            let (x, y) = (id_of(map, a.name), id_of(map, b.name));
            if x != 0 && x == y {
                return false;
            }
        }
    }
    true
}

fn integer(a: &Attr) -> Option<u32> {
    matches!(a.kind, TYPE_INT_DEC | TYPE_INT_HEX).then_some(a.data)
}

fn manifest_attrs(
    attrs: &[Attr],
    pool: &Pool,
    map: Option<&[u32]>,
    out: &mut Manifest,
    package_seen: &mut bool,
) -> Option<()> {
    if !distinct(attrs, map) {
        return None;
    }
    for a in attrs {
        let text = pool.get(usize::try_from(a.name).ok()?)?;
        if a.ns == -1 && text == "package" {
            // A literal string, not empty, with no resource ID.
            if a.kind != TYPE_STRING
                || a.raw != a.data as i32
                || *package_seen
                || id_of(map, a.name) != 0
            {
                return None;
            }
            let p = pool.get(a.data as usize)?;
            if p.is_empty() {
                return None;
            }
            out.package = p;
            *package_seen = true;
            continue;
        }
        // The name's text and its resource ID agree: an attribute named like
        // one of ours with another ID or none, or with the ID of one of
        // ours under another name, is two readers' two readings.
        let id = id_of(map, a.name);
        if !agrees(&text, id) {
            return None;
        }
        match id {
            VERSION_CODE => out.version_code = integer(a)?,
            VERSION_CODE_MAJOR => out.version_code_major = integer(a)?,
            VERSION_NAME => {
                if a.kind != TYPE_STRING || a.raw != a.data as i32 {
                    return None;
                }
                out.version_name = Some(pool.get(a.data as usize)?);
            }
            _ => {}
        }
    }
    Some(())
}

fn sdk_attrs(attrs: &[Attr], pool: &Pool, map: Option<&[u32]>, out: &mut Manifest) -> Option<()> {
    if !distinct(attrs, map) {
        return None;
    }
    for a in attrs {
        let id = id_of(map, a.name);
        if !agrees(&pool.get(usize::try_from(a.name).ok()?)?, id) {
            return None;
        }
        if id == MIN_SDK {
            // Read as signed: a negative or absurd value is no SDK level.
            let v = i32::try_from(integer(a)?).ok()?;
            if !(0..=10_000).contains(&v) {
                return None;
            }
            out.min_sdk = v as u32;
        }
    }
    Some(())
}

/// Whether a name's text and its resource ID agree about the four
/// attributes Faraday reads: each ID has its name and each name its ID.
fn agrees(text: &str, id: u32) -> bool {
    let by_text = match text {
        "versionCode" => VERSION_CODE,
        "versionCodeMajor" => VERSION_CODE_MAJOR,
        "versionName" => VERSION_NAME,
        "minSdkVersion" => MIN_SDK,
        _ => 0,
    };
    let ours = |i: u32| {
        matches!(
            i,
            VERSION_CODE | VERSION_CODE_MAJOR | VERSION_NAME | MIN_SDK
        )
    };
    by_text == id || (by_text == 0 && !ours(id))
}
