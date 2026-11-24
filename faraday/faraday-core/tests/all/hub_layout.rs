//! Where a page's panes stand: the stack and the pair at the window height
//! where they change, a flow's pair beside the guide, and the phone.

use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn sized(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

/// Where the line `text` starts, if it is drawn.
fn at(app: &mut Faraday, text: &str) -> Option<(i32, i32)> {
    let _ = app.frame();
    let found = app
        .drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == text)?;
    Some((found.1.0, found.1.1))
}

/// Check, 1920 wide and `height` tall: whether its tile is under This
/// session's card, not beside it.
fn check_stacks(height: u16) -> bool {
    let mut app = sized(1920, height, 160);
    app.press(Action::Nav(Screen::Check));
    let card = at(&mut app, EN.home_box_heading).expect("This session");
    let group = at(&mut app, EN.check_group_wallet).expect("a group of Check");
    group.1 > card.1 + 100 && group.0 < card.0 + 360
}

#[test]
fn check_turns_from_the_pair_to_the_stack_once_as_the_window_grows() {
    let mut stacked = Vec::new();
    for height in (600..=1100).step_by(50) {
        stacked.push(check_stacks(height));
    }
    assert!(!stacked[0], "a short window has the pair");
    assert!(*stacked.last().unwrap(), "a tall one has the stack");
    let turns = stacked.windows(2).filter(|w| w[0] != w[1]).count();
    assert_eq!(turns, 1, "it turns once: {stacked:?}");
}

#[test]
fn the_guide_moves_a_flows_pair_left_and_its_pane_stays_whole() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let mut app = sized(1600, 900, 160);
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![("spend.psbt".to_string(), psbt)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::StartSpend(0));
    let _ = app.frame();
    let shut = app.pane_place().expect("the flow's pane");
    app.press(Action::Guide);
    let _ = app.frame();
    let open = app.pane_place().expect("the flow's pane beside the guide");
    assert!(open.0 < shut.0, "the pair moves left: {shut:?} {open:?}");
    assert!(open.0 >= 240 + 24, "a margin from the navigation: {open:?}");
}

#[test]
fn the_phone_has_no_session_pane_on_check() {
    // Tall enough for the stack, so the width alone decides.
    let mut app = sized(480, 2400, 160);
    app.press(Action::Nav(Screen::Check));
    assert!(
        at(&mut app, EN.check_group_wallet).is_some(),
        "Check is drawn"
    );
    assert!(
        at(&mut app, EN.home_box_heading).is_none(),
        "no pane stands on the phone"
    );
}
