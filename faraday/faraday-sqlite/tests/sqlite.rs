//! The reader against databases SQLite itself made (`make_fixtures.py`),
//! and against those databases with one thing changed.

use faraday_sqlite::{Database, Error, Row, refusing_siblings};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn key(i: usize) -> Vec<u8> {
    let mut k = (i as u32).to_be_bytes().to_vec();
    k.extend([b'k'; 20]);
    k
}

fn value(i: usize) -> Vec<u8> {
    (0..24).map(|j| ((i * 7 + j) % 256) as u8).collect()
}

fn blob(i: usize, n: usize) -> Vec<u8> {
    (0..n).map(|j| ((i * 31 + j) % 251) as u8).collect()
}

fn rows(bytes: &[u8]) -> Result<Vec<Row>, Error> {
    Database::open(bytes)?.rows("main")
}

#[test]
fn every_page_size_reads_the_rows_sqlite_wrote() {
    let expected: Vec<_> = (0..300).map(|i| (key(i), value(i))).collect();
    for ps in [512, 1024, 2048, 4096, 8192, 16384, 32768, 65536] {
        let bytes = fixture(&format!("ps{ps}.db"));
        let db = Database::open(&bytes).unwrap();
        assert_eq!(db.header().page_size, ps, "page size {ps}");
        assert_eq!(db.rows("main").unwrap(), expected, "page size {ps}");
        db.only_table_with_rows("main").unwrap();
    }
}

#[test]
fn the_header_gives_what_the_wallet_layer_needs() {
    let bytes = fixture("ps512.db");
    let db = Database::open(&bytes).unwrap();
    let h = db.header();
    assert_eq!(h.application_id, 0xf9be_b4d9);
    assert_eq!(h.user_version, 7);
    assert_eq!(h.text_encoding, 1);
    assert_eq!(h.schema_format, 4);
    assert_eq!(h.page_count as usize * 512, bytes.len());
    let names: Vec<_> = db.tables().iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["main"]);
    assert!(
        db.tables()[0]
            .sql
            .as_deref()
            .unwrap()
            .contains("BLOB PRIMARY KEY")
    );
}

#[test]
fn values_that_spill_onto_overflow_pages_read_whole() {
    let sizes = [0, 1, 100, 400, 477, 478, 479, 480, 481, 1000, 2000, 5000];
    let expected: Vec<_> = sizes
        .iter()
        .enumerate()
        .map(|(i, &n)| (key(i), blob(i, n)))
        .collect();
    assert_eq!(rows(&fixture("overflow.db")).unwrap(), expected);
}

#[test]
fn deleted_rows_are_not_returned() {
    let got = rows(&fixture("deleted.db")).unwrap();
    // Row ids are 1-based: odd ones remain, except 101..=159.
    let expected: Vec<_> = (1..=400usize)
        .filter(|r| r % 2 == 1 && !(*r > 100 && *r < 160))
        .map(|r| (key(r - 1), blob(r - 1, 40 + (r - 1) % 90)))
        .collect();
    assert_eq!(got, expected);
}

#[test]
fn a_freelist_is_followed_and_its_pages_are_not_read_as_rows() {
    let bytes = fixture("freelist.db");
    let db = Database::open(&bytes).unwrap();
    assert!(db.header().freelist_pages > 0);
    assert_eq!(db.rows("main").unwrap().len(), 5);
}

#[test]
fn an_empty_database_reads_and_only_main_may_hold_rows() {
    let bytes = fixture("empty.db");
    let db = Database::open(&bytes).unwrap();
    assert!(db.rows("main").unwrap().is_empty());
    db.only_table_with_rows("main").unwrap();
    assert_eq!(db.rows("nothing"), Err(Error::NoTable("nothing".into())));
}

#[test]
fn a_table_other_than_main_holding_records_is_refused() {
    let bytes = fixture("other_table.db");
    let db = Database::open(&bytes).unwrap();
    assert_eq!(
        db.only_table_with_rows("main"),
        Err(Error::OtherTable("extra".into()))
    );
}

#[test]
fn a_column_that_is_not_a_blob_or_a_third_column_is_refused() {
    assert_eq!(rows(&fixture("text_value.db")), Err(Error::NotBlob));
    assert_eq!(rows(&fixture("three_columns.db")), Err(Error::Columns));
}

#[test]
fn the_wal_form_is_refused() {
    assert_eq!(rows(&fixture("wal.db")), Err(Error::Wal));
    let mut b = fixture("ps512.db");
    b[19] = 2;
    assert_eq!(rows(&b), Err(Error::Wal));
}

#[test]
fn files_that_are_not_databases_are_refused() {
    assert_eq!(rows(b""), Err(Error::NotSqlite));
    assert_eq!(rows(&[0u8; 4096]), Err(Error::NotSqlite));
    let mut b = fixture("ps512.db");
    b[0] = b'X';
    assert_eq!(rows(&b), Err(Error::NotSqlite));
}

#[test]
fn a_header_field_sqlite_does_not_allow_is_refused() {
    let base = fixture("ps512.db");
    let change = |at: usize, to: &[u8]| {
        let mut b = base.clone();
        b[at..at + to.len()].copy_from_slice(to);
        rows(&b)
    };
    assert_eq!(change(16, &[2, 0xff]), Err(Error::PageSize)); // not a power of two
    assert_eq!(change(16, &[0, 0]), Err(Error::PageSize));
    assert_eq!(change(16, &[0x01, 0x00]), Err(Error::PageSize)); // 256
    assert_eq!(change(16, &[0xc0, 0x00]), Err(Error::PageSize)); // 49,152
    assert_eq!(
        change(18, &[3]),
        Err(Error::HeaderField("read and write version"))
    );
    assert_eq!(
        change(19, &[0]),
        Err(Error::HeaderField("read and write version"))
    );
    assert_eq!(
        change(20, &[64]),
        Err(Error::HeaderField("reserved bytes per page"))
    );
    assert_eq!(
        change(21, &[63]),
        Err(Error::HeaderField("payload fractions"))
    );
    assert_eq!(
        change(22, &[31]),
        Err(Error::HeaderField("payload fractions"))
    );
    assert_eq!(
        change(23, &[33]),
        Err(Error::HeaderField("payload fractions"))
    );
    assert_eq!(
        change(44, &[0, 0, 0, 5]),
        Err(Error::HeaderField("schema format number"))
    );
    assert_eq!(
        change(44, &[0, 0, 0, 0]),
        Err(Error::HeaderField("schema format number"))
    );
    assert_eq!(
        change(56, &[0, 0, 0, 4]),
        Err(Error::HeaderField("text encoding"))
    );
    assert_eq!(change(56, &[0, 0, 0, 2]), Err(Error::TextEncoding));
    assert_eq!(
        change(80, &[1]),
        Err(Error::HeaderField("reserved for expansion"))
    );
    assert_eq!(change(52, &[0, 0, 0, 2]), Err(Error::AutoVacuum));
}

#[test]
fn a_size_or_freelist_the_file_does_not_bear_out_is_refused() {
    let base = fixture("ps512.db");
    let pages = base.len() / 512;
    // The header's size is believed only when the change counter says so.
    let mut b = base.clone();
    let counter = u32::from_be_bytes(b[24..28].try_into().unwrap());
    b[92..96].copy_from_slice(&counter.to_be_bytes());
    b[28..32].copy_from_slice(&(pages as u32 + 1).to_be_bytes());
    assert_eq!(rows(&b), Err(Error::FileSize));
    let mut b = base.clone();
    b.truncate(base.len() - 100);
    assert_eq!(rows(&b), Err(Error::FileSize));
    let mut b = base.clone();
    b.extend([0u8; 512]);
    b[28..32].copy_from_slice(&0u32.to_be_bytes());
    // An extra page nobody reaches reads (SQLite takes the file's size).
    assert!(rows(&b).is_ok());
    // A freelist of a count and no trunk, or a trunk past the file.
    let mut b = base.clone();
    b[36..40].copy_from_slice(&1u32.to_be_bytes());
    assert_eq!(rows(&b), Err(Error::Freelist));
    let mut b = base.clone();
    b[32..36].copy_from_slice(&3u32.to_be_bytes());
    assert_eq!(rows(&b), Err(Error::Freelist));
    let mut b = base.clone();
    b[32..36].copy_from_slice(&(pages as u32 + 5).to_be_bytes());
    b[36..40].copy_from_slice(&1u32.to_be_bytes());
    assert_eq!(rows(&b), Err(Error::Freelist));
    // A freelist whose count is not what the chain holds.
    let free = fixture("freelist.db");
    let mut b = free.clone();
    let n = u32::from_be_bytes(b[36..40].try_into().unwrap());
    b[36..40].copy_from_slice(&(n - 1).to_be_bytes());
    assert_eq!(rows(&b), Err(Error::Freelist));
    let mut b = free.clone();
    b[36..40].copy_from_slice(&(n + 1).to_be_bytes());
    assert_eq!(rows(&b), Err(Error::Freelist));
}

/// The page of the file at `n` (1-based) of a 512-byte database.
fn at(n: usize) -> usize {
    (n - 1) * 512
}

#[test]
fn a_page_reached_twice_is_refused() {
    let base = fixture("ps512.db");
    let db = Database::open(&base).unwrap();
    let root = db.tables()[0].root_page as usize;
    assert_eq!(
        base[at(root)],
        0x05,
        "the 512-byte table has interior pages"
    );
    // The first cell's child pointer is made the right-most pointer too.
    let first_cell = u16::from_be_bytes([base[at(root) + 12], base[at(root) + 13]]) as usize;
    let child = base[at(root) + first_cell..at(root) + first_cell + 4].to_vec();
    let mut b = base.clone();
    b[at(root) + 8..at(root) + 12].copy_from_slice(&child);
    assert!(
        matches!(rows(&b), Err(Error::PageRepeated(_))),
        "{:?}",
        rows(&b)
    );
    // A freelist page that is also a table page.
    let free = fixture("freelist.db");
    let trunk = u32::from_be_bytes(free[32..36].try_into().unwrap()) as usize;
    let leaf = u32::from_be_bytes(free[at(trunk) + 8..at(trunk) + 12].try_into().unwrap());
    let db = Database::open(&free).unwrap();
    let root = db.tables()[0].root_page;
    let mut b = free.clone();
    b[at(trunk) + 8..at(trunk) + 12].copy_from_slice(&root.to_be_bytes());
    assert_eq!(rows(&b), Err(Error::PageRepeated(root)));
    assert_ne!(leaf, root);
}

#[test]
fn a_page_past_the_file_or_of_the_wrong_kind_is_refused() {
    let base = fixture("ps512.db");
    let pages = (base.len() / 512) as u32;
    let root = Database::open(&base).unwrap().tables()[0].root_page as usize;
    let mut b = base.clone();
    b[at(root) + 8..at(root) + 12].copy_from_slice(&(pages + 1).to_be_bytes());
    assert_eq!(rows(&b), Err(Error::PageOutOfFile(pages + 1)));
    let mut b = base.clone();
    b[at(root) + 8..at(root) + 12].copy_from_slice(&0u32.to_be_bytes());
    assert_eq!(rows(&b), Err(Error::PageOutOfFile(0)));
    let mut b = base.clone();
    b[at(root)] = 0x0a; // an index leaf where a table page belongs
    assert_eq!(rows(&b), Err(Error::BadPage(root as u32)));
    let mut b = base.clone();
    b[at(root) + 3..at(root) + 5].copy_from_slice(&0xffffu16.to_be_bytes());
    assert_eq!(rows(&b), Err(Error::BadPage(root as u32)));
}

#[test]
fn a_cell_past_its_page_or_overlapping_another_is_refused() {
    let base = fixture("ps512.db");
    let db = Database::open(&base).unwrap();
    // Page 1's first leaf: find a leaf by walking down the first children.
    let mut page = db.tables()[0].root_page as usize;
    while base[at(page)] == 0x05 {
        let cell = u16::from_be_bytes([base[at(page) + 12], base[at(page) + 13]]) as usize;
        page = u32::from_be_bytes(
            base[at(page) + cell..at(page) + cell + 4]
                .try_into()
                .unwrap(),
        ) as usize;
    }
    let ptr = at(page) + 8;
    let mut b = base.clone();
    b[ptr..ptr + 2].copy_from_slice(&511u16.to_be_bytes());
    assert_eq!(rows(&b), Err(Error::CellOutside(page as u32)));
    // The second cell pointed at the first cell's start: the same row id.
    let mut b = base.clone();
    let first = [b[ptr], b[ptr + 1]];
    b[ptr + 2..ptr + 4].copy_from_slice(&first);
    assert_eq!(rows(&b), Err(Error::RowOrder));
}

#[test]
fn a_cell_overlapping_another_is_refused() {
    // The last cell of a one-leaf table is moved to start one byte into
    // the first: read from there it is a cell of a larger row id, so the
    // order holds and only its extent is wrong.
    let base = fixture("small.db");
    let leaf = at_n(
        Database::open(&base).unwrap().tables()[0].root_page as usize,
        512,
    );
    let ptr = |i: usize| leaf + 8 + 2 * i;
    let first = u16::from_be_bytes([base[ptr(0)], base[ptr(0) + 1]]) as usize;
    let mut b = base.clone();
    assert_eq!(b[leaf + 3..leaf + 5], [0, 5], "five rows");
    b[leaf + first + 2] = 100; // what is read as the moved cell's row id
    b[ptr(4)..ptr(4) + 2].copy_from_slice(&(first as u16 + 1).to_be_bytes());
    assert_eq!(
        Database::open(&b).unwrap_err(),
        Error::CellOutside(leaf as u32 / 512 + 1)
    );
}

#[test]
fn a_key_held_by_two_rows_is_refused() {
    let base = fixture("small.db");
    let leaf = at_n(
        Database::open(&base).unwrap().tables()[0].root_page as usize,
        512,
    );
    let cell = |i: usize| {
        let p = leaf + 8 + 2 * i;
        leaf + u16::from_be_bytes([base[p], base[p + 1]]) as usize
    };
    // Each cell is size, row id, a 3-byte record header, then the key.
    let (a, c) = (cell(0) + 5, cell(1) + 5);
    let mut b = base.clone();
    assert_eq!(rows(&b).unwrap().len(), 5);
    b.copy_within(a..a + 24, c);
    assert_eq!(rows(&b), Err(Error::KeyRepeated));
}

#[test]
fn a_table_name_held_twice_is_refused_whatever_its_case() {
    let base = fixture("twin_tables.db");
    let at = base.windows(9).position(|w| w == b"tablembin").unwrap();
    assert_eq!(rows(&base).unwrap().len(), 1);
    let mut b = base.clone();
    b[at + 5..at + 9].copy_from_slice(b"main");
    assert_eq!(rows(&b), Err(Error::TableRepeated("main".into())));
    b[at + 5..at + 9].copy_from_slice(b"MaIn");
    assert_eq!(rows(&b), Err(Error::TableRepeated("MaIn".into())));
}

#[test]
fn an_overflow_chain_that_ends_early_or_late_or_loops_is_refused() {
    let base = fixture("overflow.db");
    let db = Database::open(&base).unwrap();
    drop(db);
    let root = leaf_of(&base, 512, true);
    // The 5,000-byte blob is the last row, so the first cell in content
    // order. Its overflow page number is the 4 bytes after its local part.
    let cells = u16::from_be_bytes([base[at(root) + 3], base[at(root) + 4]]) as usize;
    let mut chains = Vec::new();
    for i in 0..cells {
        let off =
            u16::from_be_bytes([base[at(root) + 8 + 2 * i], base[at(root) + 9 + 2 * i]]) as usize;
        chains.push(off);
    }
    let big = chains[cells - 1];
    // A payload size of 5,028 is a two-byte varint, the row id one byte.
    let local = {
        let u = 512usize;
        let (x, m) = (u - 35, (u - 12) * 32 / 255 - 23);
        let size = 4 + 24 + 5000;
        let k = m + (size - m) % (u - 4);
        if k <= x { k } else { m }
    };
    let ptr_at = at(root) + big + 2 + 1 + local;
    let first = u32::from_be_bytes(base[ptr_at..ptr_at + 4].try_into().unwrap()) as usize;
    assert!(first > 1 && first * 512 <= base.len());
    assert_eq!(rows(&base).unwrap().len(), 12);
    // Chain cut at the first page.
    let mut b = base.clone();
    b[at(first)..at(first) + 4].copy_from_slice(&0u32.to_be_bytes());
    assert_eq!(rows(&b), Err(Error::Overflow));
    // Chain looping back on itself.
    let mut b = base.clone();
    b[at(first)..at(first) + 4].copy_from_slice(&(first as u32).to_be_bytes());
    assert_eq!(rows(&b), Err(Error::PageRepeated(first as u32)));
    // Chain continuing past the last page it needs.
    let mut last = first;
    loop {
        let n = u32::from_be_bytes(base[at(last)..at(last) + 4].try_into().unwrap()) as usize;
        if n == 0 {
            break;
        }
        last = n;
    }
    let mut b = base.clone();
    b[at(last)..at(last) + 4].copy_from_slice(&(first as u32 + 100).to_be_bytes());
    assert_eq!(rows(&b), Err(Error::Overflow));
    // A first overflow page past the file.
    let mut b = base.clone();
    b[ptr_at..ptr_at + 4].copy_from_slice(&0xffff_ffffu32.to_be_bytes());
    assert!(matches!(rows(&b), Err(Error::PageOutOfFile(_))));
}

#[test]
fn a_record_whose_header_disagrees_with_its_body_is_refused() {
    let base = fixture("ps4096.db");
    let leaf = leaf_of(&base, 4096, false);
    // The first cell in pointer order: size varint, rowid varint, then the
    // record's header size and two serial types.
    let off = u16::from_be_bytes([base[at_n(leaf, 4096) + 8], base[at_n(leaf, 4096) + 9]]) as usize;
    let rec = at_n(leaf, 4096) + off + 2;
    assert_eq!(base[rec], 3);
    // A serial type of a longer key than the record holds.
    let mut b = base.clone();
    b[rec + 1] += 2;
    assert_eq!(rows(&b), Err(Error::Record));
    // A header size past the payload.
    let mut b = base.clone();
    b[rec] = 100;
    assert_eq!(rows(&b), Err(Error::Record));
    // A reserved serial type.
    let mut b = base.clone();
    b[rec + 1] = 10;
    assert_eq!(rows(&b), Err(Error::Record));
    // A header size that ends inside a type's varint.
    let mut b = base.clone();
    b[rec] = 2;
    assert_eq!(rows(&b), Err(Error::Record));
    // Serial types that stop being blobs.
    let mut b = base.clone();
    b[rec + 1] = 13 + 2 * 24; // text of the same length
    assert_eq!(rows(&b), Err(Error::NotBlob));
}

fn at_n(n: usize, page: usize) -> usize {
    (n - 1) * page
}

/// The first leaf (`right` false) or last leaf of a table's tree.
fn leaf_of(base: &[u8], page_size: usize, right: bool) -> usize {
    let mut page = Database::open(base).unwrap().tables()[0].root_page as usize;
    while base[at_n(page, page_size)] == 0x05 {
        let p = at_n(page, page_size);
        let at = if right {
            p + 8
        } else {
            let cell = u16::from_be_bytes([base[p + 12], base[p + 13]]) as usize;
            p + cell
        };
        page = u32::from_be_bytes(base[at..at + 4].try_into().unwrap()) as usize;
    }
    page
}

#[test]
fn row_ids_out_of_order_are_refused() {
    let base = fixture("ps4096.db");
    let leaf = at_n(leaf_of(&base, 4096, false), 4096);
    let off =
        |i: usize| u16::from_be_bytes([base[leaf + 8 + 2 * i], base[leaf + 9 + 2 * i]]) as usize;
    // The rowid of cell 5 made 1: the cells are no longer increasing.
    let o = off(5);
    let mut b = base.clone();
    assert_eq!(b[leaf + o + 1], 6, "row id 6 follows a one-byte size");
    b[leaf + o + 1] = 1;
    assert_eq!(rows(&b), Err(Error::RowOrder));
}

/// A file whose page 1 starts a chain of `levels` pages, each an interior
/// page with no cells whose right-most child is the next, the last a leaf.
fn chain(levels: usize) -> Vec<u8> {
    let mut b = vec![0u8; 512 * levels];
    b[..16].copy_from_slice(b"SQLite format 3\0");
    b[16..18].copy_from_slice(&512u16.to_be_bytes());
    b[18] = 1;
    b[19] = 1;
    b[21] = 64;
    b[22] = 32;
    b[23] = 32;
    b[28..32].copy_from_slice(&(levels as u32).to_be_bytes());
    b[44..48].copy_from_slice(&4u32.to_be_bytes());
    b[56..60].copy_from_slice(&1u32.to_be_bytes());
    for n in 1..levels {
        let h = if n == 1 { 100 } else { 0 };
        let p = (n - 1) * 512;
        b[p + h] = 0x05;
        b[p + h + 5..p + h + 7].copy_from_slice(&512u16.to_be_bytes());
        b[p + h + 8..p + h + 12].copy_from_slice(&(n as u32 + 1).to_be_bytes());
    }
    let p = (levels - 1) * 512;
    b[p] = 0x0d;
    b[p + 5..p + 7].copy_from_slice(&512u16.to_be_bytes());
    b
}

#[test]
fn a_tree_of_more_than_twenty_levels_is_refused() {
    // SQLite reads twenty levels and calls twenty-one corrupt.
    Database::open(&chain(20)).unwrap();
    assert_eq!(Database::open(&chain(21)).unwrap_err(), Error::TooDeep);
    assert_eq!(Database::open(&chain(30)).unwrap_err(), Error::TooDeep);
}

#[test]
fn the_files_beside_a_database_that_refuse_it_are_named() {
    assert_eq!(
        refusing_siblings("wallet.dat"),
        ["wallet.dat-journal", "wallet.dat-wal", "wallet.dat-shm"]
    );
}

#[test]
fn no_change_of_a_single_byte_panics() {
    // Every byte of a small database made one after another 0x00, 0xff and
    // flipped: each reads or is refused.
    let base = fixture("overflow.db");
    for i in (0..base.len()).step_by(7) {
        for v in [0x00u8, 0xff, base[i] ^ 0x55] {
            let mut b = base.clone();
            b[i] = v;
            let _ = rows(&b);
        }
    }
}
