//! The committed warning fixtures are what the example still builds.

#[path = "../../examples/warnings.rs"]
mod warnings;

use std::path::PathBuf;

/// Every `tools/vectors/psbt/warn-*.psbt` is byte for byte what
/// `examples/warnings.rs` writes today, so the files cannot drift from
/// the code that explains them.
#[test]
fn the_committed_fixtures_are_the_ones_the_example_builds() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/psbt");
    let mut named: Vec<String> = Vec::new();
    for f in warnings::fixtures() {
        let path = dir.join(f.name);
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e}; run `just psbt-fixtures`", path.display()));
        assert_eq!(
            committed,
            f.psbt.to_base64(),
            "{} is not what the example builds; run `just psbt-fixtures`",
            path.display()
        );
        named.push(f.name.to_string());
    }
    // And nothing is committed that the example no longer builds.
    let mut orphans: Vec<String> = std::fs::read_dir(&dir)
        .expect("the vector directory")
        .map(|e| e.expect("a directory entry").file_name())
        .filter_map(|n| n.into_string().ok())
        .filter(|n| n.starts_with("warn-") && !named.contains(n))
        .collect();
    orphans.sort();
    assert!(orphans.is_empty(), "no longer built: {orphans:?}");
}
