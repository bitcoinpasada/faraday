//! Making a code through the app's events, the worker run to completion.

use aqrv::{Item, Size, Step};
use aqrv_app::state::Field;
use aqrv_app::{Action, ArgonQrVault, Page, Source};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Command, EntropyBytes, Event, Key};

fn app() -> ArgonQrVault {
    ArgonQrVault::new(|| {})
}

fn type_in(app: &mut ArgonQrVault, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Answers every request for entropy the app has made, as a shell does.
fn answer_entropy(app: &mut ArgonQrVault, seed: &mut u8) {
    while let Some(command) = app.poll_command() {
        if command == Command::RequestEntropy {
            *seed = seed.wrapping_add(1);
            let mut bytes = [0u8; 32];
            for (i, b) in bytes.iter_mut().enumerate() {
                *b = seed.wrapping_mul(31).wrapping_add(i as u8 * 7);
            }
            app.event(Event::Entropy(EntropyBytes::new(bytes)));
        }
    }
}

/// From the settings with a sample chosen, a passphrase typed twice.
fn at_settings(app: &mut ArgonQrVault, sample: usize, pass: &str) {
    app.tap(Action::Sample(sample));
    app.tap(Action::Next);
    app.tap(Action::Step(Step::Mib64));
    app.tap(Action::Field(Field::Pass));
    type_in(app, pass);
    app.tap(Action::Field(Field::Again));
    type_in(app, pass);
}

#[test]
fn a_sample_and_a_passphrase_make_a_code_the_library_opens() {
    let mut app = app();
    at_settings(&mut app, 0, "correct horse");
    app.tap(Action::Make);
    assert_eq!(app.state().page, Page::Working);
    answer_entropy(&mut app, &mut 0);
    app.finish_work();

    assert_eq!(app.state().page, Page::Code);
    let made = app.state().made.as_ref().expect("a code was made");
    assert_eq!(made.matrix.size(), 29);
    let c = Compute::new(|_| {});
    let opened = aqrv::open(&c, &made.bytes, "correct horse").expect("it opens");
    let Item::Entropy(entropy) = opened.item else {
        panic!("a seed was sealed");
    };
    let Item::Entropy(sample) = app.state().item().expect("the sample") else {
        panic!("a seed sample");
    };
    assert_eq!(*entropy, *sample);
    assert!(aqrv::open(&c, &made.bytes, "wrong horse").is_err());
}

#[test]
fn an_item_too_long_for_the_small_size_cannot_choose_it() {
    let mut app = app();
    app.tap(Action::Own);
    type_in(&mut app, &"a".repeat(40));
    app.tap(Action::Next);

    assert_eq!(app.state().page, Page::Settings);
    assert_eq!(app.state().source, Source::Own);
    assert_eq!(app.state().size, Size::Large);
    app.tap(Action::Size(Size::Small));
    assert_eq!(app.state().size, Size::Large);
}

#[test]
fn cancel_while_working_shows_no_code() {
    let mut app = app();
    at_settings(&mut app, 4, "a passphrase");
    app.tap(Action::Make);
    answer_entropy(&mut app, &mut 0);
    app.tap(Action::Cancel);
    assert_eq!(app.state().page, Page::Settings);

    // Whenever the run finishes on its thread, its code is not shown.
    for _ in 0..60 {
        app.event(Event::Tick { now_ms: 0 });
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(app.state().made.is_none());
    assert_eq!(app.state().page, Page::Settings);
}
