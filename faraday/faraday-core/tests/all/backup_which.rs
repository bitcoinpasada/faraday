//! Which copy was this? after the guided flow's plan changed: a question
//! asked of the places before is not answered into the places now.

use crate::backup_flow_wallet::single_key_plan;
use crate::common::*;
use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen};

/// The checklist's words, typed back again.
fn type_words_back(app: &mut Faraday, fp: [u8; 4]) {
    let keys = &app.session.keys;
    let key = keys.position(faraday_bip::keys::Fingerprint(fp)).unwrap();
    let text = keys.reveal(key, faraday_keys::Reveal::Words).to_string();
    // The checklist opens on the first copy; a second press would close it.
    let copy = faraday_core::plan::Item::Copy(0);
    if app.backup().and_then(|b| b.open) != Some(copy) {
        app.press(Action::BStep(copy));
    }
    app.press(Action::BCheck);
    testkit::type_words(app, &text);
}

/// Back up's plan, on its board.
fn open_board(app: &mut Faraday) {
    app.press(Action::BPlan);
    if app.backup().unwrap().q != Some(faraday_core::qstep::BOARD) {
        app.press(Action::BQ(faraday_core::qstep::BOARD));
    }
}

#[test]
fn an_answer_to_which_copy_after_the_plan_changed_dates_no_old_place() {
    use faraday_core::board::{BoardPress, Thing, Zone};
    let mut shell = Shell::new();
    let (mut app, _, fp) = single_key_plan(&mut shell, false, true, true);
    // Three places that each keep the words, set on the plan's board.
    open_board(&mut app);
    board(&mut app, BoardPress::AddPlace);
    put(&mut app, Thing::Seed(0), Zone::Paper(2));
    put(&mut app, Thing::Seed(0), Zone::Paper(1));
    app.press(Action::Nav(Screen::Backup));
    app.press(Action::BChecklist);
    type_words_back(&mut app, fp);
    assert!(
        shown(&mut app, "Which copy was this?"),
        "asked which of three"
    );
    assert!(app.offers(Action::BWhich(2)));
    // The plan changes on the plan's board: Place 2 is taken away, and the
    // old third place is now the second.
    open_board(&mut app);
    board(&mut app, BoardPress::RemovePlace(1));
    app.press(Action::Nav(Screen::Backup));
    app.press(Action::BChecklist);
    // The question was for the places before: no answer of it dates the
    // copy now at the place that was the third.
    assert!(!shown(&mut app, "Which copy was this?"));
    assert!(!app.offers(Action::BWhich(1)));
    // Pressing the old answer dates nothing, however it is pressed.
    app.press(Action::BWhich(1));
    // The place that was the third, now Place 2, shows no date.
    let g = faraday_core::glance::of_now(&testkit::compute(), &app, 0).expect("a chart");
    let faraday_core::glance::Backup::Plan { nodes, .. } = &g.backup else {
        panic!("no plan on the chart");
    };
    assert!(
        nodes
            .iter()
            .filter(|n| n.at != faraday_core::plan::At::Place(0))
            .all(|n| n.checked.iter().all(Option::is_none)),
        "{nodes:?}"
    );
}
