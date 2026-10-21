//! The committed warning and threshold fixtures are what the examples
//! still build.

#[path = "../examples/warnings.rs"]
mod warnings;

#[path = "../examples/threshold.rs"]
mod threshold;

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

/// The threshold wallet's record and its three share files are byte for
/// byte what `examples/threshold.rs` writes today, so the group a person
/// loads and the shares that sign for it stay one wallet.
#[test]
fn the_committed_threshold_fixtures_are_the_ones_the_example_builds() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/psbt");
    let mut files = vec![(
        threshold::RECORD_NAME,
        format!("{}\n", threshold::record().to_text()),
    )];
    for (name, words) in threshold::SHARE_NAMES.iter().zip(threshold::share_words()) {
        files.push((*name, format!("{words}\n")));
    }
    for (name, body) in files {
        let path = dir.join(name);
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e}; run `just psbt-fixtures`", path.display()));
        assert_eq!(
            committed,
            body,
            "{} is not what the example builds; run `just psbt-fixtures`",
            path.display()
        );
    }
}

/// The spend fixtures are what the example builds from Core's funded
/// PSBT: the carry file the first location wrote and the finished
/// transaction the second one did, byte for byte.
#[test]
fn the_committed_threshold_spend_is_the_one_the_example_builds() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/psbt");
    let carry = std::fs::read(dir.join(threshold::CARRY_NAME)).expect("the carry fixture");
    assert_eq!(
        carry,
        threshold::carry_file(&dir),
        "{} is not what the example builds; run `just psbt-fixtures`",
        dir.join(threshold::CARRY_NAME).display()
    );
    let signed =
        std::fs::read_to_string(dir.join(threshold::SIGNED_NAME)).expect("the signed fixture");
    assert_eq!(
        signed,
        threshold::signed(&dir).0.to_base64(),
        "{} is not what the example builds; run `just psbt-fixtures`",
        dir.join(threshold::SIGNED_NAME).display()
    );
    let transcript = std::fs::read_to_string(dir.join(threshold::TRANSCRIPT_NAME))
        .expect("the transcript fixture");
    assert_eq!(transcript, threshold::transcript(&dir));
}
