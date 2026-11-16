//! Review states what decides when a transaction can confirm: a time lock
//! as a date in a Caution and a height lock as a plain row; a relative
//! lock on the input it sits on; and whether the transaction can be
//! replaced.

use crate::common::{add_key, texts};
use faraday_core::strings::{EN, fill};
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, StorageEvent};
use faraday_psbt::bitcoin::{Sequence, absolute::LockTime, transaction::Version};
use faraday_psbt::{Level, Psbt, WarningKind};

fn kit(id: &str) -> Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

/// The spending wallet and its key loaded, and `psbt` open on Review.
fn reviewing(psbt: &Psbt) -> Faraday {
    let k = kit("spending");
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (
                faraday_text::Text::of("spending-wallet.txt"),
                k.descriptor.into_bytes(),
            ),
            (faraday_text::Text::of("spend.psbt"), psbt.to_bytes()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    add_key(&mut app, "bacon");
    let at = app.inbox().iter().position(|i| i.name == "spend.psbt");
    app.press(Action::StartSpend(at.unwrap()));
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    assert_eq!(app.spend().unwrap().open, Some(step::TRANSACTION));
    app
}

fn screen(app: &mut Faraday) -> String {
    texts(app).join(" ")
}

fn spend(version: i32, lock: LockTime, sequence: Sequence) -> Psbt {
    let mut psbt = testkit::unsigned(&kit("spending")).unwrap();
    let tx = &mut psbt.inner_mut().unsigned_tx;
    tx.version = Version(version);
    tx.lock_time = lock;
    for i in &mut tx.input {
        i.sequence = sequence;
    }
    psbt
}

#[test]
fn a_time_lock_is_a_caution_stated_as_a_date() {
    // 2030-01-01 00:00:00 UTC.
    let lock = LockTime::from_consensus(1_893_456_000);
    let mut app = reviewing(&spend(2, lock, Sequence::ENABLE_RBF_NO_LOCKTIME));
    let i = &app.spend().unwrap().inspection;
    assert!(
        i.warnings
            .iter()
            .any(|w| w.kind == WarningKind::LocktimeInFuture && w.level == Level::Caution)
    );
    let shown = screen(&mut app);
    let want = fill(EN.psbt_locktime_time, &["2030-01-01 00:00"]);
    assert!(shown.contains(&want), "{shown}");
}

#[test]
fn a_height_lock_is_a_plain_row_and_no_caution() {
    let lock = LockTime::from_consensus(2_000_000);
    let mut app = reviewing(&spend(2, lock, Sequence::ENABLE_RBF_NO_LOCKTIME));
    assert!(
        !app.spend()
            .unwrap()
            .inspection
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::LocktimeInFuture)
    );
    let shown = screen(&mut app);
    let want = fill(EN.psbt_locktime_block, &["2000000"]);
    assert!(shown.contains(EN.step_locked_until), "{shown}");
    assert!(shown.contains(&want), "{shown}");
}

#[test]
fn a_relative_lock_is_shown_on_its_input() {
    let mut app = reviewing(&spend(2, LockTime::ZERO, Sequence::from_height(65_535)));
    let shown = screen(&mut app);
    let wait = fill(EN.psbt_wait_blocks, &["65535"]);
    let want = fill(EN.psbt_warn_relative_lock, &["0", &wait]);
    assert!(shown.contains(&want), "{shown}");
}

#[test]
fn a_spend_no_input_marks_replaceable_reads_final() {
    let mut app = reviewing(&spend(2, LockTime::ZERO, Sequence::MAX));
    let shown = screen(&mut app);
    assert!(shown.contains(EN.step_final), "{shown}");
    assert!(!shown.contains(EN.step_replaceable), "{shown}");
}

#[test]
fn a_spend_that_signals_replacement_reads_replaceable() {
    let mut app = reviewing(&spend(2, LockTime::ZERO, Sequence::ENABLE_RBF_NO_LOCKTIME));
    let shown = screen(&mut app);
    assert!(shown.contains(EN.step_replaceable), "{shown}");
}
