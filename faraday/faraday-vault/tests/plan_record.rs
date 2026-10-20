//! A backup plan record written by a test build while the access layers
//! were in (field 6, an access layer's name, `docs/DECISIONS.md` F38)
//! still opens: the vault holding it is read, not refused.

use faraday_vault::records::{Contents, Record, field, kind};

#[test]
fn a_plan_record_with_an_access_layers_name_still_opens() {
    let mut plan = Record::new(kind::PLAN)
        .with(field::PLAN_WALLET, b"wallet")
        .with(field::PLAN_ANSWERS, b"places 2\nlayers 1\n");
    plan.push(field::PLAN_PLACE, b"Home");
    plan.push(6, b"Family");
    let written = Contents {
        records: vec![plan],
    };
    let plain = written.encode(4096).expect("a vault holding it is written");
    let read = Contents::decode(&plain).expect("and read back");
    assert_eq!(read.records.len(), 1);
    assert_eq!(read.records[0].text(field::PLAN_PLACE), Some("Home"));
}
