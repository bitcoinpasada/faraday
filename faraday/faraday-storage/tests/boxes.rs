//! What the app keeps across a lock, written to a directory: the keys that
//! wait for their backup go first, whole or not at all, so that a save that
//! fails part way still leaves them.

use std::fs;

use faraday_storage::Boxes;

#[test]
fn a_save_that_fails_part_way_leaves_the_gate_first_and_whole() {
    let dir = std::env::temp_dir().join(format!("faraday-boxes-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let mut boxes = Boxes::Dir(dir.clone());
    // The state before: an older gate.
    let ok = boxes.save(
        Vec::new(),
        Vec::new(),
        vec![("backup-gate".into(), b"old".to_vec())],
    );
    assert!(ok.is_ok());
    // A save that fails on a name it cannot write, listed after the gate.
    let failed = boxes.save(
        vec![("inbox-file".into(), vec![1])],
        Vec::new(),
        vec![
            ("settings".into(), b"s".to_vec()),
            ("bad/name".into(), b"x".to_vec()),
            ("backup-gate".into(), b"gpg\tabc\tKey\n".to_vec()),
        ],
    );
    assert!(failed.is_err());
    assert_eq!(
        fs::read(dir.join("kept/backup-gate")).unwrap(),
        b"gpg\tabc\tKey\n",
        "the new gate was written before the failure"
    );
    // A temporary file left by a save that was cut off goes at the next
    // save and at a restore.
    fs::write(dir.join("kept/.backup-gate.tmp"), b"half").unwrap();
    let _ = boxes.restore();
    assert!(
        !dir.join("kept/.backup-gate.tmp").exists(),
        "gone at restore"
    );
    fs::write(dir.join("kept/.backup-gate.tmp"), b"half").unwrap();
    let _ = boxes.save(Vec::new(), Vec::new(), Vec::new());
    assert!(!dir.join("kept/.backup-gate.tmp").exists(), "gone at save");
    // A write that fails leaves no temporary file: the gate is a directory.
    fs::create_dir_all(dir.join("kept/backup-gate")).unwrap();
    let failed = boxes.save(
        Vec::new(),
        Vec::new(),
        vec![("backup-gate".into(), b"x".to_vec())],
    );
    assert!(failed.is_err());
    assert!(
        !dir.join("kept/.backup-gate.tmp").exists(),
        "gone after a failure"
    );
    let _ = fs::remove_dir_all(dir.join("kept/backup-gate"));
    // A save with no gate takes the old one away.
    let _ = boxes.save(Vec::new(), Vec::new(), Vec::new());
    assert!(!dir.join("kept/backup-gate").exists());
    let _ = fs::remove_dir_all(&dir);
}
