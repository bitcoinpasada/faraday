//! The recovery key's sheet under the leak scan (`docs/SECRET-LEAKS.md`):
//! a known key is parsed and put on the sheet under an allocator that
//! looks for it in every block freed. A binary of its own: a binary has one
//! global allocator.

#[global_allocator]
static ALLOC: faraday_leak_scan::LeakScan = faraday_leak_scan::LeakScan;

use faraday_leak_scan::Form;
use faraday_starter::api::{Computer, Os, RecoveryKey};
use faraday_starter::notes::Notes;

const KEY: &str = "205568-492019-108251-336446-650375-210331-584155-047520";

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: the Starter's tests, which draw nothing"
)]
fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}

fn dell() -> Computer {
    Computer {
        os: Os::Windows,
        maker: "Dell Inc.".into(),
        model: "Latitude 5440".into(),
        family: "Latitude".into(),
        uefi: true,
        arm: false,
        secure_boot: Some(true),
        tpm_unlock: false,
    }
}

fn forms() -> Vec<Form> {
    let (a, b) = KEY.split_at(4 * 7);
    vec![
        Form::new("key", KEY.as_bytes().to_vec()),
        Form::new("key, first four groups", a.as_bytes().to_vec()),
        Form::new("key, last four groups", b.as_bytes().to_vec()),
        Form::new("key, groups joined", KEY.replace('-', "").into_bytes()),
    ]
}

/// The key as the Starter holds it, from parsed to dropped: the key, the
/// value built from it for the sheet and the rows are wiped, so no block
/// freed holds the whole key. The sheet's writer (`faraday-paper`'s
/// `notes`) still frees its line-sized copies of the key's halves
/// unwiped; they are not the whole key, and are left to that crate.
#[test]
fn no_block_freed_holds_the_whole_key_while_the_sheet_is_made() {
    let notes = Notes::for_computer(&dell()).expect("notes");
    let c = compute();
    let forms = forms();
    let armed = faraday_leak_scan::arm(&forms);
    let key = RecoveryKey::parse("C:", KEY).expect("a key");
    let pdf = notes.pdf_with_key(&c, &key);
    drop(key);
    drop(pdf);
    let report = armed.finish();
    let whole: Vec<_> = report
        .freed
        .iter()
        .filter(|h| h.form == "key" || h.form == "key, groups joined")
        .collect();
    assert!(whole.is_empty(), "{whole:?}");
}

/// The finished PDF, which carries the key, is wiped when it is dropped.
#[test]
fn the_pdf_with_the_key_is_wiped_when_dropped() {
    let notes = Notes::for_computer(&dell()).expect("notes");
    let c = compute();
    let key = RecoveryKey::parse("C:", KEY).expect("a key");
    let pdf = notes.pdf_with_key(&c, &key);
    let forms = forms();
    let armed = faraday_leak_scan::arm(&forms);
    drop(pdf);
    let report = armed.finish();
    assert!(report.freed.is_empty(), "{:?}", report.freed);
}
