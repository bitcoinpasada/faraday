//! None of faraday-learn's own pages carries a firmware sketch's marker
//! (`docs/HUB.md` §7.8, F18): the pages that do are faraday-guides',
//! which is why the desktop app, the stick and the Pi, which read only
//! this crate, never draw one.

use faraday_learn::{ids, is_marker, page};

#[test]
fn none_of_its_own_pages_has_a_sketch_marker() {
    let mut checked = 0;
    for id in ids() {
        let p = page(id).expect("a page for every id");
        for section in p.sections {
            for para in section.paragraphs {
                assert!(!is_marker(para), "{id}: {para}");
                checked += 1;
            }
        }
    }
    assert!(checked > 0);
}
