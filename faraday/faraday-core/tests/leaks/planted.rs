//! The check's own behaviour: a copy of the test seed made on purpose and
//! dropped without wiping is reported, with its form and where it was
//! freed; the same copy wiped first is not.

use crate::forms::{PASSPHRASE, PHRASE, seed_forms};
use crate::{arm, forms};
use faraday_leak_scan::When;
use zeroize::Zeroize;

#[test]
fn an_unwiped_copy_of_the_seed_dropped_is_reported() {
    let forms = seed_forms(PASSPHRASE);
    let armed = arm(&forms);
    let copy = std::hint::black_box(PHRASE.as_bytes().to_vec());
    drop(copy);
    let report = armed.finish();
    assert_eq!(report.freed.len(), 1, "{:?}", report.freed);
    let hit = &report.freed[0];
    assert_eq!(
        (hit.form.as_str(), hit.offset, hit.when),
        ("phrase text", 0, When::Freed)
    );
    assert!(
        hit.backtrace()
            .contains("an_unwiped_copy_of_the_seed_dropped_is_reported"),
        "the backtrace names the test that dropped it"
    );
    println!("{hit:?}\n{}", hit.backtrace());
}

#[test]
fn a_copy_wiped_before_it_is_dropped_is_not_reported() {
    let forms = seed_forms(PASSPHRASE);
    let armed = arm(&forms);
    let mut copy = std::hint::black_box(PHRASE.as_bytes().to_vec());
    copy.zeroize();
    drop(copy);
    assert!(armed.finish().freed.is_empty());
}

#[test]
fn a_copy_kept_is_found_by_the_scan_of_live_blocks() {
    let forms = seed_forms(PASSPHRASE);
    let armed = arm(&forms);
    let kept = std::hint::black_box(forms::PASSPHRASE.as_bytes().to_vec());
    let live = armed.scan_live();
    assert!(
        live.iter()
            .any(|h| h.form == "passphrase (NFC)" && h.size == kept.len())
    );
    drop(kept);
    drop(armed);
}
