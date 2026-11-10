//! A read-only reader of SQLite database files, of the part Bitcoin Core
//! writes (F229): the 100-byte header, the table B-tree with its interior
//! and leaf pages, overflow pages, varints and the record format, in the
//! rollback-journal form. The file format is SQLite's own, published at
//! <https://www.sqlite.org/fileformat2.html>:
//!
//! - section 1.2, the database header (page size, versions, reserved
//!   space, payload fractions, change counter, size in pages, freelist,
//!   schema format, text encoding, user version, application id);
//! - section 1.3 and 1.5, the page types and the b-tree page header;
//!   section 1.6, cell format and the payload that spills to overflow
//!   pages; section 2.1 and 2.3, varints and the record format with its
//!   serial types; section 2.6, the freelist; section 2.5, the schema
//!   table `sqlite_schema` on page 1.
//!
//! The reader holds no secrets and has no `unsafe`. It reads what it is
//! given and refuses what it does not follow; every refusal is an
//! [`Error`] variant: the WAL form, a header field SQLite does not allow,
//! a database size or freelist the file does not bear out, a page reached
//! twice, a cell or overflow chain past its page or the file, a record
//! whose header disagrees with its body, a column that is not a blob, a
//! table other than the wanted one holding records, a tree deeper than
//! [`MAX_DEPTH`]. Index trees are not read, and the pages of an index are
//! not checked against the others.
//!
//! A lock or hot journal beside the file is for the caller to check where
//! it can see the folder (Files): [`refusing_siblings`] names the files.
//! This crate is given bytes only.

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::fmt;
use std::ops::Range;

/// The most levels a b-tree may have, the root counting as one: SQLite's
/// own limit (`BTCURSOR_MAX_DEPTH`), which calls a tree of 21 levels corrupt.
pub const MAX_DEPTH: usize = 20;

/// The largest file read: the lock-byte page lies at 1 GiB.
pub const MAX_FILE: usize = 1 << 30;

const MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// The names of the files beside a database that mean it is in use or
/// holds committed data not yet in it: a hot rollback journal, a
/// write-ahead log and its shared memory. A caller that sees the folder
/// refuses to read the database when any of them exists.
pub fn refusing_siblings(file_name: &str) -> [String; 3] {
    [
        format!("{file_name}-journal"),
        format!("{file_name}-wal"),
        format!("{file_name}-shm"),
    ]
}

/// Why a file is not read. Each refusal is one variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Shorter than the header or not starting with SQLite's magic string.
    NotSqlite,
    /// The write-ahead-log form (read or write version 2).
    Wal,
    /// A page size SQLite does not allow (a power of two from 512 to
    /// 32,768, or 1 for 65,536).
    PageSize,
    /// A header field SQLite does not allow, or a form not read (named).
    HeaderField(&'static str),
    /// A text encoding other than UTF-8.
    TextEncoding,
    /// A database with pointer-map pages (auto-vacuum).
    AutoVacuum,
    /// A file over 1 GiB.
    TooLarge,
    /// A file whose length is not whole pages or disagrees with the size
    /// in the header.
    FileSize,
    /// A freelist that disagrees with the header or the file.
    Freelist,
    /// A page reached a second time (by a tree, an overflow chain or the
    /// freelist).
    PageRepeated(u32),
    /// A page number that is 0 or past the end of the file.
    PageOutOfFile(u32),
    /// A page that is not a table b-tree page of the kind needed, or whose
    /// header, cell pointers or content area are inconsistent.
    BadPage(u32),
    /// A cell that reaches past its page, overlaps another or lies outside
    /// the content area.
    CellOutside(u32),
    /// An overflow chain that ends early, late or past the file.
    Overflow,
    /// A record whose header disagrees with its body, or a reserved serial
    /// type.
    Record,
    /// A column that is not a blob where one is required.
    NotBlob,
    /// A record of other than two columns where two are required.
    Columns,
    /// Row keys not in increasing order or outside what their parent
    /// allows.
    RowOrder,
    /// A tree of more than [`MAX_DEPTH`] levels.
    TooDeep,
    /// Two tables of one name (names compared without regard to ASCII
    /// case, as SQLite does).
    TableRepeated(String),
    /// A table holding two rows with the same key.
    KeyRepeated,
    /// A `sqlite_schema` row that is not as SQLite writes it.
    SchemaRow,
    /// No table of that name.
    NoTable(String),
    /// A table holding records other than the one allowed.
    OtherTable(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

/// What the header says, after it passed every check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// The page size in bytes.
    pub page_size: u32,
    /// Bytes at the end of each page that are not used.
    pub reserved_bytes: u8,
    /// Pages in the file.
    pub page_count: u32,
    /// The file change counter.
    pub change_counter: u32,
    /// Pages on the freelist.
    pub freelist_pages: u32,
    /// The schema cookie.
    pub schema_cookie: u32,
    /// The schema format number, 1 to 4.
    pub schema_format: u32,
    /// The text encoding: 1 is UTF-8 (the only one read).
    pub text_encoding: u32,
    /// `PRAGMA user_version`.
    pub user_version: i32,
    /// `PRAGMA application_id`.
    pub application_id: u32,
    /// The SQLite version that last wrote the file.
    pub sqlite_version: u32,
}

/// A row of a table of two blob columns: its key and its value.
pub type Row = (Vec<u8>, Vec<u8>);

/// A table in `sqlite_schema`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The table's name.
    pub name: String,
    /// The page its b-tree starts on.
    pub root_page: u32,
    /// The `CREATE TABLE` text, if any.
    pub sql: Option<String>,
}

/// A database read from bytes, checked as far as its tables go.
#[derive(Debug)]
pub struct Database<'a> {
    pages: Pages<'a>,
    header: Header,
    tables: Vec<Table>,
}

struct Pages<'a> {
    data: &'a [u8],
    page_size: usize,
    usable: usize,
    count: u32,
}

impl fmt::Debug for Pages<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pages({} of {})", self.count, self.page_size)
    }
}

/// Which pages have been reached.
struct Seen(Vec<bool>);

impl Seen {
    fn mark(&mut self, page: u32) -> Result<(), Error> {
        let slot = self
            .0
            .get_mut(page as usize)
            .filter(|_| page != 0)
            .ok_or(Error::PageOutOfFile(page))?;
        if *slot {
            return Err(Error::PageRepeated(page));
        }
        *slot = true;
        Ok(())
    }
}

type Visit<'f> = dyn FnMut(i64, Vec<u8>) -> Result<(), Error> + 'f;

fn be16(b: &[u8], at: usize) -> usize {
    usize::from(u16::from_be_bytes([b[at], b[at + 1]]))
}

fn be32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// A varint at `at`: its value and its length (section 2.1).
fn varint(b: &[u8], at: usize) -> Option<(u64, usize)> {
    let mut v = 0u64;
    for i in 0..9 {
        let byte = *b.get(at.checked_add(i)?)?;
        if i == 8 {
            return Some(((v << 8) | u64::from(byte), 9));
        }
        v = (v << 7) | u64::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            return Some((v, i + 1));
        }
    }
    None
}

fn parse_header(b: &[u8]) -> Result<(Header, Pages<'_>), Error> {
    if b.len() < 100 || &b[..16] != MAGIC {
        return Err(Error::NotSqlite);
    }
    let raw = be16(b, 16);
    let page_size = match raw {
        1 => 65_536,
        512..=32_768 if raw.is_power_of_two() => raw,
        _ => return Err(Error::PageSize),
    };
    if b[18] == 2 || b[19] == 2 {
        return Err(Error::Wal);
    }
    if b[18] != 1 || b[19] != 1 {
        return Err(Error::HeaderField("read and write version"));
    }
    let reserved = b[20];
    let usable = page_size - usize::from(reserved);
    if usable < 480 {
        return Err(Error::HeaderField("reserved bytes per page"));
    }
    if b[21] != 64 || b[22] != 32 || b[23] != 32 {
        return Err(Error::HeaderField("payload fractions"));
    }
    let schema_format = be32(b, 44);
    if !(1..=4).contains(&schema_format) {
        return Err(Error::HeaderField("schema format number"));
    }
    let text_encoding = be32(b, 56);
    match text_encoding {
        1 => {}
        2 | 3 => return Err(Error::TextEncoding),
        _ => return Err(Error::HeaderField("text encoding")),
    }
    if b[72..92].iter().any(|&x| x != 0) {
        return Err(Error::HeaderField("reserved for expansion"));
    }
    if be32(b, 52) != 0 || be32(b, 64) != 0 {
        return Err(Error::AutoVacuum);
    }
    if b.len() > MAX_FILE {
        return Err(Error::TooLarge);
    }
    if !b.len().is_multiple_of(page_size) {
        return Err(Error::FileSize);
    }
    // Under 2^30 bytes of at least 512: fits a u32.
    let file_pages = (b.len() / page_size) as u32;
    let change_counter = be32(b, 24);
    let stated = be32(b, 28);
    if stated != 0 && be32(b, 92) == change_counter && stated != file_pages {
        return Err(Error::FileSize);
    }
    let trunk = be32(b, 32);
    let freelist_pages = be32(b, 36);
    if (trunk == 0) != (freelist_pages == 0) || freelist_pages >= file_pages || trunk > file_pages {
        return Err(Error::Freelist);
    }
    let header = Header {
        page_size: page_size as u32,
        reserved_bytes: reserved,
        page_count: file_pages,
        change_counter,
        freelist_pages,
        schema_cookie: be32(b, 40),
        schema_format,
        text_encoding,
        user_version: be32(b, 60) as i32,
        application_id: be32(b, 68),
        sqlite_version: be32(b, 96),
    };
    Ok((
        header,
        Pages {
            data: b,
            page_size,
            usable,
            count: file_pages,
        },
    ))
}

impl Pages<'_> {
    /// The usable bytes of a page that has been marked.
    fn page(&self, n: u32) -> Result<&[u8], Error> {
        let start = (n as usize - 1) * self.page_size;
        self.data
            .get(start..start + self.usable)
            .ok_or(Error::PageOutOfFile(n))
    }

    fn freelist(&self, trunk: u32, count: u32, seen: &mut Seen) -> Result<(), Error> {
        let mut at = trunk;
        let mut found = 0u64;
        while at != 0 {
            seen.mark(at)?;
            let p = self.page(at)?;
            let next = be32(p, 0);
            let n = be32(p, 4) as usize;
            if n > self.usable / 4 - 2 {
                return Err(Error::Freelist);
            }
            found += 1 + n as u64;
            if found > u64::from(count) {
                return Err(Error::Freelist);
            }
            for i in 0..n {
                seen.mark(be32(p, 8 + 4 * i))?;
            }
            at = next;
        }
        if found != u64::from(count) {
            return Err(Error::Freelist);
        }
        Ok(())
    }

    /// The payload of a cell: its local bytes and its overflow chain.
    fn payload(
        &self,
        local: &[u8],
        total: u64,
        first: u32,
        seen: &mut Seen,
    ) -> Result<Vec<u8>, Error> {
        let mut out = Vec::with_capacity(total as usize);
        out.extend_from_slice(local);
        let mut remaining = total - local.len() as u64;
        let mut next = first;
        while remaining > 0 {
            if next == 0 {
                return Err(Error::Overflow);
            }
            seen.mark(next)?;
            let p = self.page(next)?;
            let n = remaining.min((self.usable - 4) as u64) as usize;
            out.extend_from_slice(&p[4..4 + n]);
            remaining -= n as u64;
            next = be32(p, 0);
        }
        if next != 0 {
            return Err(Error::Overflow);
        }
        Ok(out)
    }

    /// Walks a table b-tree in row order (section 1.5, 1.6). Row ids in
    /// the subtree are above `lo` and at most `hi`.
    fn walk(
        &self,
        page: u32,
        depth: usize,
        (lo, hi): (i128, i128),
        seen: &mut Seen,
        visit: &mut Visit<'_>,
    ) -> Result<(), Error> {
        if depth >= MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        seen.mark(page)?;
        let p = self.page(page)?;
        let h = if page == 1 { 100 } else { 0 };
        let kind = p[h];
        let leaf = match kind {
            0x0d => true,
            0x05 => false,
            _ => return Err(Error::BadPage(page)),
        };
        let head = if leaf { 8 } else { 12 };
        let cells = be16(p, h + 3);
        let content = match be16(p, h + 5) {
            0 => 65_536,
            c => c,
        };
        let ptr_end = h + head + 2 * cells;
        if ptr_end > self.usable || content < ptr_end || content > self.usable {
            return Err(Error::BadPage(page));
        }
        let outside = Error::CellOutside(page);
        let mut extents: Vec<(usize, usize)> = Vec::with_capacity(cells);
        let mut children: Vec<(u32, i64)> = Vec::new();
        let mut prev = lo;
        for i in 0..cells {
            let off = be16(p, h + head + 2 * i);
            if off < content || off >= self.usable {
                return Err(outside);
            }
            if leaf {
                let (size, n1) = varint(p, off).ok_or(outside.clone())?;
                let (rowid, n2) = varint(p, off + n1).ok_or(outside.clone())?;
                let rowid = rowid as i64;
                if i128::from(rowid) <= prev || i128::from(rowid) > hi {
                    return Err(Error::RowOrder);
                }
                prev = i128::from(rowid);
                if size > self.data.len() as u64 {
                    return Err(outside);
                }
                let u = self.usable as u64;
                let max_local = u - 35;
                let min_local = (u - 12) * 32 / 255 - 23;
                let local = if size <= max_local {
                    size
                } else {
                    let k = min_local + (size - min_local) % (u - 4);
                    if k <= max_local { k } else { min_local }
                } as usize;
                let start = off + n1 + n2;
                let mut end = start + local;
                let spills = size > local as u64;
                if spills {
                    end += 4;
                }
                if end > self.usable {
                    return Err(outside);
                }
                extents.push((off, end));
                let first = if spills { be32(p, start + local) } else { 0 };
                let bytes = self.payload(&p[start..start + local], size, first, seen)?;
                visit(rowid, bytes)?;
            } else {
                if off + 4 > self.usable {
                    return Err(outside);
                }
                let (key, n) = varint(p, off + 4).ok_or(outside.clone())?;
                extents.push((off, off + 4 + n));
                let key = key as i64;
                if i128::from(key) <= prev || i128::from(key) > hi {
                    return Err(Error::RowOrder);
                }
                prev = i128::from(key);
                children.push((be32(p, off), key));
            }
        }
        extents.sort_unstable();
        if extents.windows(2).any(|w| w[0].1 > w[1].0) || extents.iter().any(|e| e.1 > self.usable)
        {
            return Err(outside);
        }
        if !leaf {
            let mut below = lo;
            for (child, key) in children {
                self.walk(child, depth + 1, (below, i128::from(key)), seen, visit)?;
                below = i128::from(key);
            }
            self.walk(be32(p, h + 8), depth + 1, (below, hi), seen, visit)?;
        }
        Ok(())
    }
}

/// The columns of a record: each serial type with its body's range
/// (section 2.3). The header must agree with the body to the byte.
fn columns(p: &[u8]) -> Result<Vec<(u64, Range<usize>)>, Error> {
    let (hsize, n) = varint(p, 0).ok_or(Error::Record)?;
    if hsize < n as u64 || hsize > p.len() as u64 {
        return Err(Error::Record);
    }
    let hsize = hsize as usize;
    let head = &p[..hsize];
    let mut pos = n;
    let mut body = hsize as u64;
    let mut out = Vec::new();
    while pos < hsize {
        let (t, l) = varint(head, pos).ok_or(Error::Record)?;
        pos += l;
        let size = match t {
            0 | 8 | 9 => 0,
            1..=4 => t,
            5 => 6,
            6 | 7 => 8,
            10 | 11 => return Err(Error::Record),
            _ => (t - 12) / 2,
        };
        let end = body.checked_add(size).ok_or(Error::Record)?;
        if end > p.len() as u64 {
            return Err(Error::Record);
        }
        out.push((t, body as usize..end as usize));
        body = end;
    }
    if body != p.len() as u64 {
        return Err(Error::Record);
    }
    Ok(out)
}

fn is_blob(t: u64) -> bool {
    t >= 12 && t.is_multiple_of(2)
}

fn text(p: &[u8], c: &(u64, Range<usize>)) -> Option<String> {
    if c.0 >= 13 && c.0 % 2 == 1 {
        String::from_utf8(p[c.1.clone()].to_vec()).ok()
    } else {
        None
    }
}

fn integer(p: &[u8], c: &(u64, Range<usize>)) -> Option<i64> {
    match c.0 {
        8 => Some(0),
        9 => Some(1),
        1..=6 => {
            let b = &p[c.1.clone()];
            let mut v = if b[0] & 0x80 != 0 { -1i64 } else { 0 };
            for &x in b {
                v = (v << 8) | i64::from(x);
            }
            Some(v)
        }
        _ => None,
    }
}

impl<'a> Database<'a> {
    /// Reads a database from its bytes: the header, the freelist, the
    /// schema and every table's tree are checked (a page reached twice, a
    /// cell or overflow past the file, keys out of order), but no record
    /// of a table is read until [`rows`](Self::rows).
    pub fn open(bytes: &'a [u8]) -> Result<Database<'a>, Error> {
        let (header, pages) = parse_header(bytes)?;
        let mut seen = Seen(vec![false; header.page_count as usize + 1]);
        let mut tables = Vec::new();
        pages.walk(1, 0, (i128::MIN, i128::MAX), &mut seen, &mut |_, p| {
            let cols = columns(&p)?;
            if cols.len() != 5 {
                return Err(Error::SchemaRow);
            }
            let kind = text(&p, &cols[0]).ok_or(Error::SchemaRow)?;
            let name = text(&p, &cols[1]).ok_or(Error::SchemaRow)?;
            text(&p, &cols[2]).ok_or(Error::SchemaRow)?;
            let root = integer(&p, &cols[3]).ok_or(Error::SchemaRow)?;
            let sql = match cols[4].0 {
                0 => None,
                _ => Some(text(&p, &cols[4]).ok_or(Error::SchemaRow)?),
            };
            if kind == "table" {
                if tables
                    .iter()
                    .any(|t: &Table| t.name.eq_ignore_ascii_case(&name))
                {
                    return Err(Error::TableRepeated(name));
                }
                let root = u32::try_from(root)
                    .ok()
                    .filter(|r| (2..=header.page_count).contains(r))
                    .ok_or(Error::SchemaRow)?;
                tables.push(Table {
                    name,
                    root_page: root,
                    sql,
                });
            }
            Ok(())
        })?;
        if header.freelist_pages != 0 {
            let trunk = be32(bytes, 32);
            pages.freelist(trunk, header.freelist_pages, &mut seen)?;
        }
        for t in &tables {
            pages.walk(
                t.root_page,
                0,
                (i128::MIN, i128::MAX),
                &mut seen,
                &mut |_, _| Ok(()),
            )?;
        }
        Ok(Database {
            pages,
            header,
            tables,
        })
    }

    /// The header's fields.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// The tables of `sqlite_schema`, in its order.
    pub fn tables(&self) -> &[Table] {
        &self.tables
    }

    fn table(&self, name: &str) -> Result<&Table, Error> {
        self.tables
            .iter()
            .find(|t| t.name == name)
            .ok_or_else(|| Error::NoTable(name.to_string()))
    }

    fn walk_table(&self, t: &Table, visit: &mut Visit<'_>) -> Result<(), Error> {
        let mut seen = Seen(vec![false; self.pages.count as usize + 1]);
        self.pages
            .walk(t.root_page, 0, (i128::MIN, i128::MAX), &mut seen, visit)
    }

    /// The rows of a table of two blob columns (key, value), in row id
    /// order, as `CREATE TABLE main(key BLOB PRIMARY KEY NOT NULL, value
    /// BLOB NOT NULL)` makes it. A key seen twice is refused: the index
    /// that makes keys unique in SQLite is not read.
    pub fn rows(&self, table: &str) -> Result<Vec<Row>, Error> {
        let t = self.table(table)?;
        let mut out = Vec::new();
        let mut keys = HashSet::new();
        self.walk_table(t, &mut |_, p| {
            let cols = columns(&p)?;
            if cols.len() != 2 {
                return Err(Error::Columns);
            }
            if !cols.iter().all(|c| is_blob(c.0)) {
                return Err(Error::NotBlob);
            }
            let key = p[cols[0].1.clone()].to_vec();
            if !keys.insert(key.clone()) {
                return Err(Error::KeyRepeated);
            }
            out.push((key, p[cols[1].1.clone()].to_vec()));
            Ok(())
        })?;
        Ok(out)
    }

    /// Refuses a database in which a table other than `name` holds a
    /// record (`sqlite_sequence` and the like included), and one without
    /// the table `name`.
    pub fn only_table_with_rows(&self, name: &str) -> Result<(), Error> {
        self.table(name)?;
        for t in self.tables.iter().filter(|t| t.name != name) {
            let mut any = false;
            self.walk_table(t, &mut |_, _| {
                any = true;
                Ok(())
            })?;
            if any {
                return Err(Error::OtherTable(t.name.clone()));
            }
        }
        Ok(())
    }
}
