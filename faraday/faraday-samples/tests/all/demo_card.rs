//! The "Demo only" card (`docs/DEMO.md` §14, owner 2026-10-24): what a page
//! or a sheet offers only in Demo, or only on Faraday Desktop in place of
//! the person's own input, stands once, in the card at the window's right
//! on a wide window and in a strip at its foot on a narrow one, and
//! nowhere in the page's own column.

use crate::common::{demo_app, show};
use faraday_core::keygen::{Way, kstep};
use faraday_core::vault::{RollerAct, VaultAction as V};
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_shell_api::{App, Event, Key};

/// Demo with the samples, at `w` × `h` (one pixel a unit).
fn at(w: u16, h: u16) -> Faraday {
    let mut app = demo_app();
    show(&mut app, w, h, 160);
    app
}

/// The "Demo only" heading's box on the frame drawn now, in pixels.
fn heading(app: &mut Faraday) -> Option<(i32, i32, i32, i32)> {
    app.drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == "Demo only")
        .map(|b| b.1)
}

/// Whether `action` is offered once, in the card: not left of its inner
/// edge, and under its heading or, in a strip of buttons alone, in the
/// heading's row.
fn in_card(app: &mut Faraday, action: Action) -> bool {
    let Some((hx, hy, ..)) = heading(app) else {
        return false;
    };
    let Some((x, y, _, h)) = app.hit_box(action) else {
        return false;
    };
    app.offered_times(action) == 1 && i32::from(x) >= hx - 16 && i32::from(y + h) > hy
}

/// Whether a line holding `part` is drawn in the card, under its heading.
fn text_in_card(app: &mut Faraday, part: &str) -> bool {
    let Some((hx, hy, ..)) = heading(app) else {
        return false;
    };
    let boxes = app.drawn_text_boxes();
    let found: Vec<_> = boxes.iter().filter(|(t, _)| t.contains(part)).collect();
    !found.is_empty() && found.iter().all(|(_, b)| b.0 >= hx && b.1 > hy)
}

/// The card at the window's right on a wide window, at its foot on a
/// narrow one.
fn card_stands_where_it_should(app: &mut Faraday, (w, h): (u16, u16)) {
    let (hx, hy, ..) = heading(app).expect("the Demo only card");
    if w >= 1000 {
        assert!(hx > i32::from(w) / 2, "at the right: {hx}");
        assert!(hx < i32::from(w) - 24, "inside the margin: {hx}");
    } else {
        assert!(hy > i32::from(h) / 3, "at the foot: {hy}");
    }
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

#[test]
fn add_a_key_has_use_a_sample_and_use_faraday_in_the_card_at_1280_and_480() {
    for size in [(1280, 800), (480, 640)] {
        let mut app = at(size.0, size.1);
        app.press(Action::Entry(None));
        let _ = app.frame();
        card_stands_where_it_should(&mut app, size);
        assert!(
            in_card(&mut app, Action::UseSample),
            "{size:?}: Use a sample"
        );
        let faraday = Action::PassChip(faraday_core::demo::PassField::Entry);
        if size.0 >= 1000 {
            assert!(in_card(&mut app, faraday), "{size:?}: Use faraday");
        }
        assert!(
            !app.drawn_texts().iter().any(|t| t == "faraday"),
            "no chip inside a field"
        );
        // Filled, the card's Use a sample goes; the page holds the words.
        app.press(Action::UseSample);
        let _ = app.frame();
        assert!(!app.offers(Action::UseSample));
    }
}

#[test]
fn new_key_typed_has_its_sample_in_the_card() {
    let mut app = at(1280, 800);
    let i = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Restore from SLIP-39")
        .expect("a Restore from SLIP-39 tile");
    app.press(Action::Catalog(i as u8));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::KeyGen);
    assert!(in_card(&mut app, Action::UseSample));
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
fn new_key_says_in_the_card_which_sample_lock_in_adds() {
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
    assert!(text_in_card(&mut app, "Lock in adds sample"));
    assert!(app.offers(Action::KLock), "Lock in stays on the Key card");
    let (lx, ..) = app.hit_box(Action::KLock).expect("Lock in");
    let (hx, ..) = heading(&mut app).expect("the card");
    assert!(i32::from(lx) < hx, "Lock in is in the page's column");
}

#[test]
fn the_computers_dice_on_new_key_are_in_the_card() {
    for size in [(1280, 800), (480, 640)] {
        let mut app = at(size.0, size.1);
        app.set_roller_offered(true);
        new_key_entry(&mut app, Way::DiceHashed);
        let _ = app.frame();
        card_stands_where_it_should(&mut app, size);
        assert!(
            in_card(&mut app, Action::KRoller(RollerAct::Roll)),
            "{size:?}: the roller's Roll"
        );
    }
}

#[test]
fn vaults_sample_vault_create_a_vaults_faraday_and_its_dice_are_in_the_card() {
    let mut app = at(1280, 800);
    app.set_roller_offered(true);
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    assert!(in_card(&mut app, Action::UseSample), "the sample vault");
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    let faraday = Action::PassChip(faraday_core::demo::PassField::Phrase(0, false));
    assert!(in_card(&mut app, faraday), "Use faraday");
    app.press(Action::Vault(V::Dice(0)));
    let _ = app.frame();
    assert!(
        in_card(&mut app, Action::Vault(V::Roller(RollerAct::Roll))),
        "Generate with dice's roller"
    );
    type_in(&mut app, "123451234512345123451234512345");
    app.press(Action::Vault(V::DiceUse));
    let _ = app.frame();
    assert!(text_in_card(
        &mut app,
        "Demo puts faraday in place of these words"
    ));
}

#[test]
fn the_unlocks_faraday_is_in_the_card() {
    let mut app = at(1280, 800);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::UseSample);
    app.press(Action::Vault(V::Open(0)));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Unlock);
    assert!(in_card(
        &mut app,
        Action::PassChip(faraday_core::demo::PassField::Unlock)
    ));
}

#[test]
fn the_spend_tabs_load_sample_psbt_is_in_the_card() {
    let mut app = crate::psbt::with_savings();
    show(&mut app, 1280, 800, 160);
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(
        faraday_core::family::FamilyAction::SpendFrom(0),
    ));
    let _ = app.frame();
    assert!(in_card(&mut app, Action::SamplePsbt));
}

#[test]
fn the_scanners_sample_is_in_the_card_beside_the_sheet_at_1280_and_480() {
    for size in [(1280, 800), (480, 640)] {
        let mut app = at(size.0, size.1);
        app.press(Action::ScanSeed);
        let _ = app.frame();
        assert_eq!(app.sheet, Some(Sheet::Scan));
        card_stands_where_it_should(&mut app, size);
        assert!(in_card(&mut app, Action::ScanSample), "{size:?}");
        app.press(Action::ScanSample);
        assert_eq!(app.sheet, Some(Sheet::SeedRead), "it reads the sample");
    }
}

#[test]
fn lightning_find_an_address_and_check_my_copy_have_their_samples_in_the_card() {
    let mut app = at(1280, 800);
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    let _ = app.frame();
    assert!(in_card(&mut app, Action::UseSample), "the cipher seed");

    let mut app = at(1280, 800);
    crate::passphrase::with_spending(&mut app);
    app.press(Action::Explore);
    app.press(Action::X(faraday_core::explore::XAction::Page(
        faraday_core::explore::Page::Find,
    )));
    let _ = app.frame();
    assert!(in_card(&mut app, Action::UseSample), "Find an address");

    let mut app = at(1280, 800);
    crate::use_sample::open_words_check(&mut app);
    let _ = app.frame();
    assert!(in_card(&mut app, Action::UseSample), "Fill words");
}

#[test]
fn outside_demo_no_card_is_drawn() {
    let mut app = testkit::started();
    show(&mut app, 1280, 800, 160);
    app.set_roller_offered(true);
    app.press(Action::Entry(None));
    let _ = app.frame();
    assert!(heading(&mut app).is_none(), "Add a key on the stick");
    new_key_entry(&mut app, Way::DiceHashed);
    let _ = app.frame();
    assert!(heading(&mut app).is_none(), "New key's dice off Demo");
    assert!(
        !app.offers(Action::KRoller(RollerAct::Roll)),
        "Faraday Desktop's roller is Demo's alone"
    );
}

#[test]
fn the_card_stands_just_under_the_pills_and_24_dp_from_the_right_edge() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080)] {
        let mut app = at(w, h);
        app.press(Action::Entry(None));
        let _ = app.frame();
        let (hx, hy, ..) = heading(&mut app).expect("the card");
        assert!(hy < 96, "{w}: the heading at {hy}, just under the pills");
        let (x, _, bw, _) = app.hit_box(Action::UseSample).unwrap();
        assert!(i32::from(x + bw) <= i32::from(w) - 24, "{w}: in the margin");
        assert!(hx >= i32::from(w) - 24 - 400, "{w}: in the 400 dp card");
    }
}

#[test]
fn add_a_keys_foot_hints_stand_whole_in_the_page_beside_the_card() {
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
        let (hx, ..) = heading(&mut app).expect("the card");
        let boxes = app.drawn_text_boxes();
        let hint = boxes
            .iter()
            .find(|(t, _)| t.ends_with(end))
            .unwrap_or_else(|| panic!("the hint ending {end:?} is drawn whole"));
        assert!(
            hint.1.0 + hint.1.2 <= hx - 16,
            "{end:?} runs under the card: {:?}",
            hint.1
        );
    }
}

#[test]
fn the_rollers_strip_is_one_row_with_use_faraday_at_1280() {
    let mut app = at(1280, 800);
    app.set_roller_offered(true);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::Dice(0)));
    let _ = app.frame();
    let roll = app
        .hit_box(Action::Vault(V::Roller(RollerAct::Roll)))
        .expect("Roll");
    let faraday = Action::PassChip(faraday_core::demo::PassField::Phrase(0, false));
    let use_faraday = app.hit_box(faraday).expect("Use faraday");
    let (_, hy, ..) = heading(&mut app).expect("the strip");
    assert_eq!(roll.1, use_faraday.1, "Roll and Use faraday in one row");
    assert!(
        hy > 800 - 140,
        "the strip is one row high: its heading at {hy}"
    );
}
