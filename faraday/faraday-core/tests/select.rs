//! Selecting what is typed: a second click on a field selects all of it,
//! a drag selects the characters it went over, and Shift with an arrow
//! key extends the selection; Backspace then removes what is selected and
//! a typed character replaces it. In a passphrase's dots, and in New
//! key's box of rolls, a selection is by position, and the key made is
//! the one of what is left (`docs/NEW-WALLET.md` §13.2). A wallet's new
//! name is typed without a stray caret character in it.

use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};

fn device(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn kit_file(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn touch(app: &mut Faraday, x: u16, y: u16, phase: TouchPhase) {
    app.event(Event::Touch { x, y, phase });
}

/// A click at (x, y) at time `ms`.
fn click(app: &mut Faraday, (x, y): (u16, u16), ms: u64) {
    app.event(Event::Tick { now_ms: ms });
    touch(app, x, y, TouchPhase::Down);
    touch(app, x, y, TouchPhase::Up);
    let _ = app.frame();
}

/// The Unlock screen with a long passphrase typed.
fn typed_passphrase() -> (Faraday, (u16, u16)) {
    let mut app = device(vec![kit_file("vault.ofv")]);
    app.press(Action::Vault(V::Open(0)));
    assert_eq!(app.screen, Screen::Unlock);
    type_text(&mut app, "a long passphrase typed wrong somewhere");
    let _ = app.frame();
    let at = app
        .where_offered(Action::Vault(V::FocusPassphrase))
        .expect("the passphrase field");
    (app, at)
}

#[test]
fn a_double_click_selects_the_passphrase_and_backspace_empties_it() {
    let (mut app, at) = typed_passphrase();
    click(&mut app, at, 1_000);
    click(&mut app, at, 1_200);
    assert!(app.selected_all());
    app.event(Event::Key(Key::Backspace));
    assert!(app.vaults.passphrase.text.is_empty());
    type_text(&mut app, "next");
    assert_eq!(
        &*app.vaults.passphrase.text, "next",
        "typing goes on as before"
    );
}

/// A drag on a field from the edge before character `from` to the edge
/// before character `to`.
fn drag(app: &mut Faraday, field: Action, from: usize, to: usize) {
    let (x0, y) = app.where_typed(field, from).expect("the first edge");
    let (x1, _) = app.where_typed(field, to).expect("the last edge");
    app.event(Event::Tick { now_ms: 1_000 });
    touch(app, x0, y, TouchPhase::Down);
    touch(app, x1, y, TouchPhase::Move);
    touch(app, x1, y, TouchPhase::Up);
    let _ = app.frame();
}

#[test]
fn a_drag_across_the_field_selects_it_and_typing_replaces_it() {
    let (mut app, _) = typed_passphrase();
    let n = app.vaults.passphrase.text.chars().count();
    drag(&mut app, Action::Vault(V::FocusPassphrase), 0, n);
    assert!(app.selected_all());
    type_text(&mut app, "z");
    assert_eq!(&*app.vaults.passphrase.text, "z");
}

#[test]
fn two_clicks_far_apart_in_time_select_nothing() {
    let (mut app, at) = typed_passphrase();
    click(&mut app, at, 1_000);
    click(&mut app, at, 3_000);
    assert!(!app.selected_all());
    app.event(Event::Key(Key::Backspace));
    // The press put the caret where it landed: Backspace takes the one
    // character before it and leaves the rest as they were.
    let typed = "a long passphrase typed wrong somewhere";
    let left = app.vaults.passphrase.text.to_string();
    assert_eq!(left.len(), typed.len() - 1, "Backspace takes one character");
    assert!((0..typed.len()).any(|i| format!("{}{}", &typed[..i], &typed[i + 1..]) == left));
}

#[test]
fn a_wallets_name_is_selected_and_typed_over() {
    let mut app = device(vec![kit_file("spending-wallet.txt")]);
    app.press(Action::LoadWallet(0));
    assert_eq!(app.screen, Screen::Wallets);
    app.press(Action::Rename);
    let _ = app.frame();
    let at = app.where_offered(Action::Rename).expect("the name field");
    click(&mut app, at, 1_000);
    click(&mut app, at, 1_200);
    app.event(Event::Key(Key::Backspace));
    assert_eq!(app.renaming.as_deref(), Some(""));
    type_text(&mut app, "Groceries");
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.session.wallets[0].name, "Groceries");
}

/// Wallets, with the loaded wallet's new name typed as `name`.
fn renamed(name: &str) -> Faraday {
    let mut app = device(vec![kit_file("spending-wallet.txt")]);
    app.press(Action::LoadWallet(0));
    app.press(Action::Rename);
    let _ = app.frame();
    let at = app.where_offered(Action::Rename).expect("the name field");
    click(&mut app, at, 1_000);
    click(&mut app, at, 1_200);
    type_text(&mut app, name);
    let _ = app.frame();
    app
}

#[test]
fn a_drag_over_three_characters_and_backspace_leaves_the_rest() {
    let mut app = renamed("Groceries");
    drag(&mut app, Action::Rename, 2, 5);
    app.event(Event::Key(Key::Backspace));
    assert_eq!(app.renaming.as_deref(), Some("Grries"));
}

#[test]
fn a_character_typed_over_a_selection_replaces_it_and_typing_goes_on_after_it() {
    let mut app = renamed("Groceries");
    // Dragged right to left, the same three.
    drag(&mut app, Action::Rename, 5, 2);
    type_text(&mut app, "a");
    assert_eq!(app.renaming.as_deref(), Some("Graries"));
    type_text(&mut app, "b");
    assert_eq!(app.renaming.as_deref(), Some("Grabries"));
}

#[test]
fn shift_with_the_arrow_keys_selects_and_delete_takes_what_is_selected() {
    let mut app = renamed("Groceries");
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Left));
    app.event(Event::Shift { held: true });
    for _ in 0..3 {
        app.event(Event::Key(Key::Left));
    }
    app.event(Event::Shift { held: false });
    app.event(Event::Key(Key::Delete));
    assert_eq!(app.renaming.as_deref(), Some("Groces"));
}

#[test]
fn shift_with_a_press_selects_from_the_caret_to_the_press() {
    let mut app = renamed("Groceries");
    let at = app.where_typed(Action::Rename, 4).unwrap();
    click(&mut app, at, 5_000);
    let to = app.where_typed(Action::Rename, 7).unwrap();
    app.event(Event::Shift { held: true });
    click(&mut app, to, 7_000);
    app.event(Event::Shift { held: false });
    app.event(Event::Key(Key::Backspace));
    assert_eq!(app.renaming.as_deref(), Some("Groces"));
}

/// A tall display, so a whole card is drawn at once.
fn tall(app: &mut Faraday) {
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2400,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// A 2-of-3 Create with New key for its first slot carried past the
/// flips to the Key card.
fn key_card() -> Faraday {
    use faraday_core::create::NewKind;
    use faraday_core::keygen::Way;
    let mut app = testkit::started();
    tall(&mut app);
    app.press(Action::CreateWallet);
    let at = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    app.press(Action::CKind(at));
    app.press(Action::CNext(faraday_core::cstep::KIND));
    app.press(Action::CNext(faraday_core::cstep::QUORUM));
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 5 + i / 3) % 2 == 0));
    }
    app.press(Action::KNext);
    app
}

#[test]
fn an_edit_inside_the_passphrase_makes_the_key_of_the_words_with_the_edited_passphrase() {
    let mut app = key_card();
    let phrase = app.keygen.as_ref().unwrap().phrase().unwrap();
    app.press(Action::KPassField(0));
    type_text(&mut app, "Ride the 7 bus");
    let _ = app.frame();
    // The dots stand for the characters one to one: the tenth is the 7.
    drag(&mut app, Action::KPassField(0), 9, 10);
    type_text(&mut app, "8");
    assert_eq!(app.keygen.as_ref().unwrap().passphrase, "Ride the 8 bus");
    app.press(Action::KPassField(1));
    type_text(&mut app, "Ride the 8 bus");
    app.press(Action::KLock);
    let want = faraday_core::wallet::Session::default()
        .add_words_with(&phrase, "Ride the 8 bus", "", None)
        .unwrap()
        .0;
    let added: Vec<[u8; 4]> = app
        .session
        .keys
        .iter()
        .map(|k| k.master.fingerprint().0)
        .collect();
    assert_eq!(added, vec![want]);
}

/// New key on its own, hashed dice typed into the box.
fn dice_box() -> Faraday {
    use faraday_core::keygen::Way;
    let mut app = testkit::started();
    tall(&mut app);
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    app.press(Action::KWords(12));
    app.press(Action::KGroup(1));
    app.press(Action::KWay(Way::DiceHashed.index()));
    app.press(Action::KNext);
    assert!(app.keygen.as_ref().unwrap().typing, "typed into the box");
    app
}

/// The words New key makes once the entries are in.
fn words(app: &mut Faraday) -> String {
    app.press(Action::KNext);
    app.keygen.as_ref().unwrap().phrase().unwrap().to_string()
}

#[test]
fn deleting_rolls_four_to_six_leaves_the_others_in_order_and_the_key_is_the_rolls_left() {
    let mut app = dice_box();
    type_text(&mut app, "1234561234");
    let _ = app.frame();
    drag(&mut app, Action::KTyping(true), 3, 6);
    app.event(Event::Key(Key::Backspace));
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.entered, "1231234", "the others, in order");
    assert_eq!(k.dice.len(), 7, "the rolls the key is made from");
    // The rest of the rolls, typed where the caret was left, then the
    // words: those of the rolls the box shows.
    let need = k.progress().1;
    let more: String = (0..need - 7)
        .map(|i| char::from(b'1' + (i * 7 % 6) as u8))
        .collect();
    type_text(&mut app, &more);
    let shown = format!("123{more}1234");
    assert_eq!(app.keygen.as_ref().unwrap().entered, shown.as_str());
    let made = words(&mut app);
    let mut direct = dice_box();
    type_text(&mut direct, &shown);
    assert_eq!(made, words(&mut direct));
}
