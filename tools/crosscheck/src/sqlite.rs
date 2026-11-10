//! The SQLite reader, `faraday_sqlite`: the tables and the rows of a
//! database's table of two blob columns, as Python's `sqlite3` reads them.

use faraday_sqlite::Database;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex};

/// Handles `sqlite.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        // The table names and the rows of `main`.
        "read" => {
            let image = bytes(req, "db");
            let db = Database::open(&image).map_err(|e| format!("{e:?}"))?;
            let rows = db.rows("main").map_err(|e| format!("{e:?}"))?;
            let tables: Vec<&str> = db.tables().iter().map(|t| t.name.as_str()).collect();
            Ok(json!({
                "tables": tables,
                "rows": rows.iter().map(|(k, v)| json!([hex(k), hex(v)])).collect::<Vec<_>>(),
            }))
        }
        _ => Err(format!("unknown sqlite op {name}")),
    }
}
