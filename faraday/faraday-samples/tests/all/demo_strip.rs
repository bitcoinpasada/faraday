//! Demo's aids in the host's strip (`docs/DEMO.md` §14, `docs/DESKTOP-HOST.md`
//! §3): what a page or a sheet offers only in Demo, or only on Faraday
//! Desktop in place of the person's own input, stands once in the strip
//! at the window's foot (in More where the strip has no room) and nowhere
//! in the app: no "Demo only" card or column, no chip in a page.

use crate::common::{demo_app, show, strip, strip_offers};
use faraday_core::keygen::{Way, kstep};
use faraday_core::vault::{RollerAct, VaultAction as V};
use faraday_core::{Action, Faraday, Host, HostPress, Screen, Sheet, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

/// Demo with the samples, at `w` × `h` (one pixel a unit).
fn at(w: u16, h: u16) -> Faraday {
    let mut app = demo_app();
    show(&mut app, w, h, 160);
    app
}

/// Whether `action` is offered by the strip, and not by the app's own
/// frame (which draws no demo-only control).
fn in_strip_only(app: &mut Faraday, action: Action) -> bool {
    let in_strip = strip_offers(app, action);
    let _ = app.frame();
    in_strip && !app.offers(action)
}

/// Nothing of the old card is drawn in the app.
fn app_draws_no_card(app: &mut Faraday) {
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t == "Demo only"),
        "no Demo only card in the app"
    );
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// The host holding `app` in a window `w` × `h`, a frame drawn.
fn hosted(app: Faraday, w: u16, h: u16) -> Host {
    let mut host = Host::new(app);
    host.event(Event::Display(DisplayInfo {
        width: w,
        height: h,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = host.frame();
    host
}

#[test]
fn add_a_key_has_use_a_sample_and_use_faraday_in_the_strip_at_1280_and_480() {
    for size in [(1280, 800), (480, 640)] {
        let mut app = at(size.0, size.1);
        app.press(Action::Entry(None));
        let _ = app.frame();
        app_draws_no_card(&mut app);
        assert!(
            in_strip_only(&mut app, Action::UseSample),
            "{size:?}: Use a sample"
        );
        let faraday = Action::PassChip(faraday_core::demo::PassField::Entry);
        assert!(in_strip_only(&mut app, faraday), "{size:?}: Use faraday");
        assert!(
            !app.drawn_texts().iter().any(|t| t == "faraday"),
            "no chip inside a field"
        );
        // Filled, the strip's Use a sample goes; the page holds the words.
        app.press(Action::UseSample);
        let _ = app.frame();
        assert!(!strip_offers(&mut app, Action::UseSample));
    }
}

#[test]
fn new_key_typed_has_its_sample_in_the_strip() {
    let mut app = at(1280, 800);
    let i = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Restore from SLIP-39")
        .expect("a Restore from SLIP-39 tile");
    app.press(Action::Catalog(i as u8));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::KeyGen);
    assert!(in_strip_only(&mut app, Action::UseSample));
}

/// New key at twelve words by `way`, its entry open.
fn new_key_entry(app: &mut Faraday, way: Way) {
    app.press(Action::KeyGen(None));
    app.press(Action::KWords(12));
    app.press(Action::KNext);
    app.press(Action::KWay(way.index()));
    app.press(Action::KNext);
    assert_eq!(app.keygen().unwrap().open, Some(kstep::ENTER));
}

#[test]
fn new_key_says_in_the_strip_which_sample_lock_in_adds() {
    let mut app = at(1280, 800);
    new_key_entry(&mut app, Way::Coins);
    for i in 0..400 {
        let k = app.keygen().unwrap();
        if k.open == Some(kstep::KEY) {
            break;
        }
        let (have, need) = k.progress();
        if have < need {
            app.press(Action::KFlip((i * 5 + i / 3) % 2 == 0));
        } else {
            app.press(Action::KNext);
        }
    }
    assert_eq!(app.keygen().unwrap().open, Some(kstep::KEY));
    let _ = app.frame();
    assert!(
        strip(&mut app)
            .iter()
            .any(|t| t.contains("Lock in adds sample")),
        "the strip says which sample Lock in adds"
    );
    assert!(
        !app.drawn_texts()
            .iter()
            .any(|t| t.contains("Lock in adds sample")),
        "the app does not"
    );
    assert!(app.offers(Action::KLock), "Lock in stays on the Key card");
}

#[test]
fn the_computers_dice_on_new_key_are_in_the_strip() {
    for size in [(1280, 800), (480, 640)] {
        let mut app = at(size.0, size.1);
        app.set_roller_offered(true);
        new_key_entry(&mut app, Way::DiceHashed);
        let _ = app.frame();
        app_draws_no_card(&mut app);
        assert!(
            in_strip_only(&mut app, Action::KRoller(RollerAct::Roll)),
            "{size:?}: the roller's Roll"
        );
    }
}

#[test]
fn vaults_sample_vault_create_a_vaults_faraday_and_its_dice_are_in_the_strip() {
    let mut app = at(1280, 800);
    app.set_roller_offered(true);
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    assert!(
        in_strip_only(&mut app, Action::UseSample),
        "the sample vault"
    );
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    let faraday = Action::PassChip(faraday_core::demo::PassField::Phrase(0, false));
    assert!(in_strip_only(&mut app, faraday), "Use faraday");
    app.press(Action::Vault(V::Dice(0)));
    let _ = app.frame();
    assert!(
        in_strip_only(&mut app, Action::Vault(V::Roller(RollerAct::Roll))),
        "Generate with dice's roller"
    );
    type_in(&mut app, "123451234512345123451234512345");
    app.press(Action::Vault(V::DiceUse));
    let _ = app.frame();
    assert!(
        strip(&mut app)
            .iter()
            .any(|t| t == "Demo puts faraday in place of these words"),
        "the strip says what Demo puts in place of the words"
    );
}

#[test]
fn the_unlocks_faraday_is_in_the_strip() {
    let mut app = at(1280, 800);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::UseSample);
    app.press(Action::Vault(V::Open(0)));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Unlock);
    assert!(in_strip_only(
        &mut app,
        Action::PassChip(faraday_core::demo::PassField::Unlock)
    ));
}

#[test]
fn the_spend_tabs_load_sample_psbt_is_in_the_strip() {
    let mut app = crate::psbt::with_savings();
    show(&mut app, 1280, 800, 160);
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(
        faraday_core::family::FamilyAction::SpendFrom(0),
    ));
    let _ = app.frame();
    assert!(in_strip_only(&mut app, Action::SamplePsbt));
}

#[test]
fn the_scanners_sample_is_in_the_strip_beside_the_sheet_at_1280_and_480() {
    for size in [(1280, 800), (480, 640)] {
        let mut app = at(size.0, size.1);
        app.press(Action::ScanSeed);
        let _ = app.frame();
        assert_eq!(app.sheet, Some(Sheet::Scan));
        app_draws_no_card(&mut app);
        assert!(in_strip_only(&mut app, Action::ScanSample), "{size:?}");
        app.press(Action::ScanSample);
        assert_eq!(app.sheet, Some(Sheet::SeedRead), "it reads the sample");
    }
}

#[test]
fn lightning_find_an_address_and_check_my_copy_have_their_samples_in_the_strip() {
    let mut app = at(1280, 800);
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    let _ = app.frame();
    assert!(
        in_strip_only(&mut app, Action::UseSample),
        "the cipher seed"
    );

    let mut app = at(1280, 800);
    crate::passphrase::with_spending(&mut app);
    app.press(Action::Explore);
    app.press(Action::X(faraday_core::explore::XAction::Page(
        faraday_core::explore::Page::Find,
    )));
    let _ = app.frame();
    assert!(
        in_strip_only(&mut app, Action::UseSample),
        "Find an address"
    );

    let mut app = at(1280, 800);
    crate::use_sample::open_words_check(&mut app);
    let _ = app.frame();
    assert!(in_strip_only(&mut app, Action::UseSample), "Fill words");
}

#[test]
fn outside_demo_the_strip_offers_no_aid() {
    let mut app = testkit::started();
    show(&mut app, 1280, 800, 160);
    app.set_roller_offered(true);
    app.press(Action::Entry(None));
    let _ = app.frame();
    assert!(
        !strip_offers(&mut app, Action::UseSample),
        "Add a key on the stick"
    );
    new_key_entry(&mut app, Way::DiceHashed);
    let _ = app.frame();
    assert!(
        !strip_offers(&mut app, Action::KRoller(RollerAct::Roll)),
        "Faraday Desktop's roller is Demo's alone"
    );
    app_draws_no_card(&mut app);
}

#[test]
fn the_aids_stand_in_the_strip_at_the_windows_foot() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080)] {
        let mut app = at(w, h);
        app.press(Action::Entry(None));
        let host = hosted(app, w, h);
        let (x, y) = host
            .where_offered(HostPress::App(Action::UseSample))
            .expect("Use a sample in the strip");
        assert!(
            i32::from(y) > i32::from(h) - 56,
            "{w}: Use a sample at {y}, in the 56 dp strip at the foot"
        );
        assert!(x < w, "{w}: inside the window");
    }
}

#[test]
fn add_a_keys_foot_hints_stand_whole_in_the_page() {
    use faraday_core::forms::Form;
    let mut app = at(1280, 800);
    app.press(Action::Entry(None));
    let _ = app.frame();
    for (form, end) in [
        (None, "Esc goes back"),
        (Some(Form::Slip39), "makes the key"),
    ] {
        if let Some(f) = form {
            let at = Form::ALL.iter().position(|g| *g == f).unwrap() as u8;
            app.press(Action::EntryForm(at));
            let _ = app.frame();
        }
        let boxes = app.drawn_text_boxes();
        let hint = boxes
            .iter()
            .find(|(t, _)| t.ends_with(end))
            .unwrap_or_else(|| panic!("the hint ending {end:?} is drawn whole"));
        assert!(
            hint.1.0 + hint.1.2 <= 1280,
            "{end:?} runs off the window: {:?}",
            hint.1
        );
    }
}

#[test]
fn the_rollers_roll_and_use_faraday_stand_in_one_row_of_the_strip_at_1280() {
    let mut app = at(1280, 800);
    app.set_roller_offered(true);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::Dice(0)));
    let host = hosted(app, 1280, 800);
    let roll = host
        .where_offered(HostPress::App(Action::Vault(V::Roller(RollerAct::Roll))))
        .expect("Roll");
    let faraday = Action::PassChip(faraday_core::demo::PassField::Phrase(0, false));
    let use_faraday = host
        .where_offered(HostPress::App(faraday))
        .expect("Use faraday");
    assert_eq!(roll.1, use_faraday.1, "Roll and Use faraday in one row");
}
