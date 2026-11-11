//! Opening a code through the app's events, the worker run to completion.

use aqrv::Step;
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

/// Makes a code from the first sample and saves it: the bytes of the
/// `.aqrv` file and of the picture the app handed the shell to write.
fn made_and_saved() -> (Vec<u8>, Vec<u8>) {
    let mut app = ArgonQrVault::new(|| {});
    app.tap(Action::Sample(0));
    app.tap(Action::Next);
    app.tap(Action::Step(Step::Mib64));
    for (field, _) in [(Field::Pass, 0), (Field::Again, 1)] {
        app.tap(Action::Field(field));
        type_in(&mut app, PASS);
    }
    app.tap(Action::Make);
    let mut seed = 0u8;
    while let Some(command) = app.poll_command() {
        if command == Command::RequestEntropy {
            seed = seed.wrapping_add(1);
            let bytes = std::array::from_fn(|i| seed.wrapping_mul(31).wrapping_add(i as u8 * 7));
            app.event(Event::Entropy(EntropyBytes::new(bytes)));
        }
    }
    app.finish_work();
    assert_eq!(app.state().page, Page::Code);
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
    (
        file.expect("a file was written"),
        picture.expect("a picture was written"),
    )
}

fn drop_file(app: &mut ArgonQrVault, bytes: Vec<u8>) {
    app.event(Event::File {
        kind: FileKind::Any,
        bytes,
    });
    app.finish_work();
}

fn unlock(app: &mut ArgonQrVault, pass: &str) {
    app.tap(Action::Field(Field::OpenPass));
    type_in(app, pass);
    app.tap(Action::Unlock);
    app.finish_work();
}

fn shows_a_seed(app: &ArgonQrVault) -> bool {
    let shown = app.state().open.shown.as_ref().expect("an item is shown");
    let words = shown.lines.iter().find(|(l, _)| *l == Label::Words);
    let print = shown.lines.iter().any(|(l, _)| *l == Label::Fingerprint);
    print && words.is_some_and(|(_, w)| w.split(' ').count() == 12)
}

#[test]
fn a_saved_code_opens_again_with_its_passphrase_and_shows_the_item() {
    let (file, _) = made_and_saved();
    let mut app = ArgonQrVault::new(|| {});
    drop_file(&mut app, file);
    assert_eq!(app.state().open.stage, OpenStage::Asking);
    assert!(app.state().open.header.is_some());
    unlock(&mut app, PASS);
    assert_eq!(app.state().open.stage, OpenStage::Opened);
    assert!(shows_a_seed(&app));
}

#[test]
fn a_saved_picture_opens_too() {
    let (_, picture) = made_and_saved();
    let mut app = ArgonQrVault::new(|| {});
    drop_file(&mut app, picture);
    assert_eq!(app.state().open.stage, OpenStage::Asking);
    unlock(&mut app, PASS);
    assert!(shows_a_seed(&app));
}

#[test]
fn a_wrong_passphrase_does_not_open() {
    let (file, _) = made_and_saved();
    let mut app = ArgonQrVault::new(|| {});
    drop_file(&mut app, file);
    unlock(&mut app, "wrong horse");
    assert_eq!(app.state().open.stage, OpenStage::DoesNotOpen);
    assert!(app.state().open.shown.is_none());
}

#[test]
fn a_file_that_is_not_a_code_is_refused_before_any_passphrase_is_asked_for() {
    let mut app = ArgonQrVault::new(|| {});
    drop_file(&mut app, b"just some text".to_vec());
    assert_eq!(app.state().open.stage, OpenStage::NotACode);
    app.tap(Action::Field(Field::OpenPass));
    type_in(&mut app, PASS);
    app.tap(Action::Unlock);
    app.finish_work();
    assert_eq!(app.state().open.stage, OpenStage::NotACode);
}

#[test]
fn a_typed_path_asks_the_shell_for_that_file() {
    let mut app = ArgonQrVault::new(|| {});
    app.tap(Action::Field(Field::Path));
    type_in(&mut app, "/tmp/code.aqrv");
    app.tap(Action::ReadPath);
    let mut asked = None;
    while let Some(command) = app.poll_command() {
        if let Command::ReadFile { name, .. } = command {
            asked = Some(name);
        }
    }
    assert_eq!(asked.as_deref(), Some("/tmp/code.aqrv"));
}
