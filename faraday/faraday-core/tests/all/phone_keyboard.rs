//! On a one-column layout (a phone, the 7-inch panel) a press on a bar
//! or field that takes typing brings the on-screen keyboard up.

use faraday_core::edit::Here;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, DisplayInfo, Event, TouchPhase};

/// A phone and the 7-inch panel.
const WINDOWS: [(u16, u16, u16); 2] = [(1081, 2400, 420), (1024, 600, 170)];

fn window((width, height, dpi): (u16, u16, u16)) -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

fn tap(app: &mut Faraday, action: Action) {
    let _ = app.frame();
    let (x, y) = app
        .where_offered(action)
        .unwrap_or_else(|| panic!("{action:?} is not on screen"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
    let _ = app.frame();
}

#[test]
fn a_press_on_a_bar_or_field_that_takes_typing_brings_the_keyboard_up_and_again_once_put_away() {
    use crate::common::type_text;
    for w in WINDOWS {
        // Tools' Find a tool, a bar with no field to focus.
        let mut app = window(w);
        app.press(Action::Nav(Screen::Catalog));
        let _ = app.frame();
        assert!(app.keyboard_top().is_none(), "{w:?}: down first");
        tap(&mut app, Action::TypeHere(Here::Find));
        assert!(app.keyboard_top().is_some(), "{w:?}: Find a tool");

        // BIP-85's index, a field already typed into: put away, a press
        // brings the keyboard back.
        let mut app = window(w);
        app.press(Action::Entry(None));
        type_text(
            &mut app,
            &faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[0].0),
        );
        app.press(Action::EntryAdd);
        app.press(Action::Bip85);
        app.press(Action::PApp(0));
        app.press(Action::PNext);
        let index = Action::TypeHere(Here::Index);
        tap(&mut app, index);
        assert!(app.keyboard_top().is_some(), "{w:?}: the index");
        tap(&mut app, Action::Osk(faraday_ui::keyboard::OskPress::Hide));
        assert!(app.keyboard_top().is_none(), "{w:?}: put away");
        tap(&mut app, index);
        assert!(app.keyboard_top().is_some(), "{w:?}: the index again");

        // Lightning's aezeed.
        let mut app = window(w);
        app.press(Action::Lightning);
        app.press(Action::LAezeed);
        tap(&mut app, Action::TypeHere(Here::Aezeed));
        assert!(app.keyboard_top().is_some(), "{w:?}: the aezeed");

        // Sweep's sheet: its field was focused as it opened, so the
        // keyboard is down until the field is pressed.
        let mut app = window(w);
        app.press(Action::Sweep(faraday_core::seeds::sweep::SweepAction::Open));
        let _ = app.frame();
        assert!(app.keyboard_top().is_none(), "{w:?}: sweep down first");
        tap(
            &mut app,
            Action::Sweep(faraday_core::seeds::sweep::SweepAction::Field),
        );
        assert!(app.keyboard_top().is_some(), "{w:?}: the sweep field");

        // The guide's contents, a sheet whose Find takes typing at once.
        let mut app = window(w);
        app.press(Action::Guide);
        app.press(Action::GuideContents);
        let _ = app.frame();
        assert!(app.keyboard_top().is_none(), "{w:?}: guide down first");
        tap(&mut app, Action::TypeHere(Here::GuideFind));
        assert!(app.keyboard_top().is_some(), "{w:?}: the guide's Find");
    }
}

#[test]
fn randomness_dice_are_pressed_as_faces_or_typed_on_the_keyboard_a_press_brings_up() {
    use faraday_core::fresh::OwnSource;
    use faraday_core::fresh::page::Act;
    use faraday_ui::keyboard::OskPress;
    for w in WINDOWS {
        let rolls = |app: &mut Faraday, n: &str| {
            let _ = app.frame();
            app.drawn_texts().iter().any(|t| t.contains(n))
        };
        let mut app = window(w);
        app.press(Action::Nav(Screen::Randomness));
        app.press(Action::Own(Act::Source(OwnSource::Dice)));
        let _ = app.frame();
        assert!(app.keyboard_top().is_none(), "{w:?}: down first");
        tap(&mut app, Action::Own(Act::Roll(3)));
        assert!(rolls(&mut app, "1 dice roll"), "{w:?}: a face pressed");

        tap(&mut app, Action::Own(Act::Typing(true)));
        assert!(app.keyboard_top().is_some(), "{w:?}: Typed brings it up");
        tap(&mut app, Action::Osk(OskPress::Char('4')));
        assert!(rolls(&mut app, "2 dice rolls"), "{w:?}: a roll typed");
        tap(&mut app, Action::Osk(OskPress::Hide));
        assert!(app.keyboard_top().is_none(), "{w:?}: put away");
        let (x, y, fw, fh) = app.focused_field().expect("the rolls' box");
        for phase in [TouchPhase::Down, TouchPhase::Up] {
            app.event(Event::Touch {
                x: (x + fw / 2) as u16,
                y: (y + fh / 2) as u16,
                phase,
            });
        }
        let _ = app.frame();
        let _ = app.frame();
        assert!(app.keyboard_top().is_some(), "{w:?}: the box brings it up");
    }
}

#[test]
fn a_field_in_a_sheet_that_scrolls_stands_whole_above_the_keyboard() {
    use faraday_core::kef_form::KefAct;
    use faraday_core::kef_tool::{KefToolAction as K, Pick};
    let reach = |app: &mut Faraday, a: Action| {
        for _ in 0..20 {
            let _ = app.frame();
            if app.where_offered(a).is_some() {
                break;
            }
            app.event(Event::Scroll {
                x: 400,
                y: 300,
                dy: 150,
            });
        }
    };
    for w in WINDOWS {
        let mut app = window(w);
        app.press(Action::Nav(Screen::Catalog));
        let tile = faraday_core::catalog::TILES
            .iter()
            .position(|t| t.name == "Krux code")
            .unwrap();
        app.press(Action::Catalog(tile as u8));
        app.press(Action::KefTool(K::Pick(Pick::Type)));
        faraday_core::testkit::type_words(&mut app, "a note");
        app.press(Action::KefTool(K::Next));
        let options = Action::KefTool(K::Form(KefAct::Options));
        reach(&mut app, options);
        tap(&mut app, options);
        // Options is taller than the 7-inch panel: Rounds is scrolled to
        // and pressed.
        let rounds = Action::KefTool(K::Form(KefAct::Rounds));
        reach(&mut app, rounds);
        tap(&mut app, rounds);
        let top = app.keyboard_top().expect("the keyboard is up");
        let (_, y, _, h) = app.focused_field().expect("the field");
        assert!(
            y + h <= i32::from(top),
            "{w:?}: field {y}+{h}, keyboard {top}"
        );
    }
}

#[test]
fn my_seed_makes_my_wallets_passphrase_fields_bring_the_keyboard_up_when_pressed() {
    use faraday_core::seed_check::SeedAction as S;
    use faraday_ui::keyboard::OskPress;
    for w in WINDOWS {
        let mut app = window(w);
        app.press(Action::SeedCheck(S::Open));
        tap(&mut app, Action::SeedCheck(S::Type));
        faraday_core::testkit::type_words(
            &mut app,
            &faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[0].0),
        );
        tap(&mut app, Action::SeedCheck(S::Words));
        // The step's fade runs out before its card is pressed.
        for _ in 0..40 {
            let _ = app.frame();
            let now = app.now_ms() + 16;
            app.event(Event::Tick { now_ms: now });
        }
        tap(&mut app, Action::SeedCheck(S::AddPass));
        tap(&mut app, Action::Osk(OskPress::Hide));
        assert!(app.keyboard_top().is_none(), "{w:?}: put away");
        tap(&mut app, Action::SeedCheck(S::PassField(0)));
        assert!(app.keyboard_top().is_some(), "{w:?}: the passphrase");
        tap(&mut app, Action::Osk(OskPress::Hide));
        tap(&mut app, Action::SeedCheck(S::PassField(1)));
        assert!(app.keyboard_top().is_some(), "{w:?}: the second field");
    }
}

#[test]
fn a_new_keys_typed_rolls_bring_the_keyboard_up_when_the_typed_button_or_the_box_is_pressed() {
    use faraday_core::keygen::{Start, Way};
    use faraday_ui::keyboard::OskPress;
    for w in WINDOWS {
        let mut app = window(w);
        app.press(Action::Entry(None));
        app.press(Action::KeyGen(None));
        app.press(Action::KStart(Start::Make.index()));
        app.press(Action::KWords(12));
        app.press(Action::KNext);
        app.press(Action::KWay(Way::Coins.index()));
        app.press(Action::KNext);
        app.press(Action::KByDie(true));
        let _ = app.frame();
        assert!(app.keyboard_top().is_none(), "{w:?}: down first");
        tap(&mut app, Action::KTyping(true));
        assert!(app.keyboard_top().is_some(), "{w:?}: Typed brings it up");
        app.press(Action::Osk(OskPress::Hide));
        let _ = app.frame();
        assert!(app.keyboard_top().is_none(), "{w:?}: put away");
        let (x, y, fw, fh) = app.focused_field().expect("the rolls' box");
        for phase in [TouchPhase::Down, TouchPhase::Up] {
            app.event(Event::Touch {
                x: (x + fw / 2) as u16,
                y: (y + fh / 2) as u16,
                phase,
            });
        }
        let _ = app.frame();
        let _ = app.frame();
        assert!(app.keyboard_top().is_some(), "{w:?}: the box brings it up");
    }
}
