//! The SQLite reader on any bytes: `Database::open`, then every table
//! read as two blob columns and checked for rows, and the header and table
//! list asked for. Nothing panics, every read is an answer or an `Error`,
//! and the same bytes opened a second time give the same rows.

#![no_main]

use faraday_sqlite::{Database, refusing_siblings};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(db) = Database::open(data) else {
        return;
    };
    let _ = db.header();
    for table in db.tables() {
        let again = Database::open(data).expect("bytes that opened open again");
        assert_eq!(db.rows(&table.name), again.rows(&table.name));
        let _ = refusing_siblings(&table.name);
    }
    let _ = db.only_table_with_rows("main");
});
