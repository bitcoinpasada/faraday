//! A stick visit's copy: a picture holding a SeedQR that the person did
//! not tick is dropped, and nothing of the picture or its code is left
//! freed unwiped.

use crate::forms::{PHRASE, seed_forms};
use crate::{arm, finish};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::{Action, Screen, StickInfo, StorageCommand, StorageEvent, testkit};
use faraday_leak_scan::Form;
use faraday_shell_api::App;

/// The SeedQR digits of `PHRASE`: each word's index as four digits.
fn digits() -> Vec<u8> {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, PHRASE).expect("the vector's words");
    m.indices()
        .iter()
        .map(|i| format!("{i:04}"))
        .collect::<String>()
        .into_bytes()
}

#[test]
fn a_seedqr_picture_left_on_its_default_tick_and_dropped_leaves_no_freed_copy() {
    let digits = std::mem::ManuallyDrop::new(digits());
    let mut forms = seed_forms("");
    forms.push(Form::new("SeedQR digits", digits.to_vec()));
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("a"),
        label: faraday_text::Text::of("A"),
        boot: false,
        files: vec![(faraday_text::Text::of("photo.png"), 9)],
        vaults: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(app.visit().inn.contains("photo.png"), "ticked by default");
    app.press(Action::VisitCopy);
    let mut asked = false;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::ReadQr { name, .. } = c {
            asked = true;
            app.storage(StorageEvent::QrRead {
                name,
                bytes: zeroize::Zeroizing::new(b"the picture".to_vec()),
                payloads: zeroize::Zeroizing::new(vec![digits.to_vec()]),
            });
        }
    }
    assert!(asked, "the picture was read for its codes");
    let _ = app.frame();
    assert!(app.inbox().is_empty(), "the SeedQR came in unasked");
    finish("a SeedQR picture dropped on a visit", armed, false);
}
