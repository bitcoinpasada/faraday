//! The 32-byte size through the app's events, the worker run to completion.

use aqrv::{Size, Step};
use aqrv_app::state::Field;
use aqrv_app::worker::Label;
use aqrv_app::{Action, ArgonQrVault, OpenStage, Page};
use faraday_shell_api::{App, Command, EntropyBytes, Event, FileKind, Key};

const PASS: &str = "correct horse";

fn type_in(app: &mut ArgonQrVault, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn at_settings(app: &mut ArgonQrVault, sample: usize) {
    app.tap(Action::Sample(sample));
    app.tap(Action::Next);
    app.tap(Action::Step(Step::Mib64));
    for field in [Field::Pass, Field::Again] {
        app.tap(Action::Field(field));
        type_in(app, PASS);
    }
}

fn answer_entropy(app: &mut ArgonQrVault) {
    while let Some(command) = app.poll_command() {
        if command == Command::RequestEntropy {
            let bytes = std::array::from_fn(|i| 0x21_u8.wrapping_add(i as u8 * 5));
            app.event(Event::Entropy(EntropyBytes::new(bytes)));
        }
    }
}

/// A 12-word seed made at the 32-byte size and saved: the file's bytes
/// and the picture's.
fn made_tiny() -> (Vec<u8>, Vec<u8>) {
    let mut app = ArgonQrVault::new(|| {});
    at_settings(&mut app, 0);
    app.tap(Action::Size(Size::Tiny));
    app.tap(Action::Make);
    answer_entropy(&mut app);
    app.finish_work();
    assert_eq!(app.state().page, Page::Code);
    let made = app.state().made.as_ref().expect("a code was made");
    assert_eq!(made.size, Size::Tiny);
    assert_eq!(made.matrix.size(), 25);
    assert_eq!(made.text.as_deref().map(str::len), Some(47));
    app.tap(Action::SaveFile);
    app.tap(Action::SavePicture);
    let (mut file, mut picture) = (None, None);
    while let Some(command) = app.poll_command() {
        if let Command::WriteFile { kind, bytes, .. } = command {
            match kind {
                FileKind::Png => picture = Some(bytes),
                _ => file = Some(bytes),
            }
        }
    }
    (file.expect("a file"), picture.expect("a picture"))
}

fn opens_to_the_seed(bytes: Vec<u8>) {
    let mut app = ArgonQrVault::new(|| {});
    app.event(Event::File {
        kind: FileKind::Any,
        bytes,
    });
    app.finish_work();
    assert_eq!(app.state().open.stage, OpenStage::Asking);
    app.tap(Action::Field(Field::OpenPass));
    type_in(&mut app, PASS);
    app.tap(Action::Unlock);
    app.finish_work();
    assert_eq!(app.state().open.stage, OpenStage::Opened);
    let shown = app.state().open.shown.as_ref().expect("an item is shown");
    let words = shown.lines.iter().find(|(l, _)| *l == Label::Words);
    assert!(words.is_some_and(|(_, w)| w.split(' ').count() == 12));
}

#[test]
fn a_seed_made_at_the_32_byte_size_saves_as_32_bytes_and_opens_again() {
    let (file, _) = made_tiny();
    assert_eq!(file.len(), 32);
    opens_to_the_seed(file);
}

#[test]
fn the_32_byte_codes_picture_opens_too() {
    let (_, picture) = made_tiny();
    opens_to_the_seed(picture);
}

#[test]
fn a_24_word_seed_cannot_be_made_at_the_32_byte_size() {
    let mut app = ArgonQrVault::new(|| {});
    at_settings(&mut app, 1);
    assert_ne!(app.state().size, Size::Tiny);
    app.tap(Action::Size(Size::Tiny));
    assert_ne!(app.state().size, Size::Tiny);
    app.tap(Action::Make);
    answer_entropy(&mut app);
    app.finish_work();
    let made = app.state().made.as_ref().expect("a code was made");
    assert_ne!(made.size, Size::Tiny);
}
