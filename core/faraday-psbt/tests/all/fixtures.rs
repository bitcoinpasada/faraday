//! The committed warning and signer-check fixtures are what the examples
//! still build.

#[path = "../../examples/signer.rs"]
mod signer;
#[path = "../../examples/warnings.rs"]
mod warnings;

use std::path::PathBuf;

/// Every `tools/vectors/psbt/warn-*.psbt` is byte for byte what
/// `examples/warnings.rs` writes today, and every `signer-*.psbt` what
/// `examples/signer.rs` writes, so the files cannot drift from
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
    // The signer check's: the two files Bitcoin Core made are inputs of the
    // example, read and not rebuilt.
    let core = ["signer-core-signed.psbt", "signer-core-final.psbt"];
    for f in signer::fixtures() {
        let path = dir.join(f.name);
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e}; run `just psbt-fixtures`", path.display()));
        assert_eq!(
            committed,
            f.text,
            "{} is not what the example builds; run `just psbt-fixtures`",
            path.display()
        );
        named.push(f.name.to_string());
    }
    named.extend(core.iter().map(|n| n.to_string()));
    // And nothing is committed that the examples no longer build.
    let mut orphans: Vec<String> = std::fs::read_dir(&dir)
        .expect("the vector directory")
        .map(|e| e.expect("a directory entry").file_name())
        .filter_map(|n| n.into_string().ok())
        .filter(|n| (n.starts_with("warn-") || n.starts_with("signer-")) && !named.contains(n))
        .collect();
    orphans.sort();
    assert!(orphans.is_empty(), "no longer built: {orphans:?}");
}
