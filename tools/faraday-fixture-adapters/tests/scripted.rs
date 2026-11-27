//! A journey's scripted adapters: what a product is given first and what an ask is answered with.

use std::fs;

use faraday_crypto::Secret;
use faraday_fixture_adapters::{Fixtures, Scripted};
use faraday_port::event::{Event, Input, Key};
use faraday_port::storage::{OnTaken, StickId, StorageAnswer, StorageAsk, Written};
use faraday_port::{Adapters, Answer, Ask, AskId, Effect};
use faraday_text::Text;

/// A kit of one stick in a folder of its own, taken away with the guard.
struct Kit(std::path::PathBuf);

impl Kit {
    fn new(name: &str) -> Kit {
        let dir = std::env::temp_dir().join(format!("fixtures-{name}-{}", std::process::id()));
        let stick = dir.join("stick");
        fs::create_dir_all(&stick).expect("the kit folder");
        fs::write(stick.join("old.txt"), b"old").expect("a file");
        Kit(dir)
    }
}

impl Drop for Kit {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Invariant: The product receives `Start` first and once, then the sticks the kit holds, then what the script gives, in order.
/// Regression: The scripted adapter polls a scripted key before `Start`, or starts twice.
/// Owner: root
#[test]
fn start_comes_first_and_once() {
    let mut fixtures = Fixtures::device(&Kit::new("start").0);
    fixtures.input(&[Scripted::Key(Key::Enter), Scripted::Text(Text::of("ab"))]);
    let mut seen = Vec::new();
    while let Some(event) = fixtures.poll() {
        seen.push(event);
    }
    assert!(matches!(seen[0], Event::Start(_)));
    assert_eq!(
        seen.iter().filter(|e| matches!(e, Event::Start(_))).count(),
        1
    );
    let keys: Vec<Key> = seen
        .iter()
        .filter_map(|e| match e {
            Event::Input(Input::Key(key)) => Some(*key),
            _ => None,
        })
        .collect();
    assert_eq!(keys, [Key::Enter, Key::Char('a'), Key::Char('b')]);
}

/// Invariant: A write the product asks for lands on the fixture stick and is listed by name and hash, and the product receives the answer to that ask.
/// Regression: The scripted adapter answers a write but does not list it, or answers another ask.
/// Owner: root
#[test]
fn a_write_is_answered_and_listed() {
    let mut fixtures = Fixtures::device(&Kit::new("write").0);
    let _start = fixtures.poll();
    let id = AskId {
        generation: 1,
        n: 1,
    };
    fixtures.carry(Effect::Ask(
        id,
        Ask::Storage(StorageAsk::Write {
            stick: StickId(Text::of("stick")),
            name: Text::of("signed.psbt"),
            bytes: Secret::new(b"psbt".to_vec()),
            copies: Vec::new(),
            replace: false,
            renamed: false,
            on_taken: OnTaken::Ask,
        }),
    ));
    let answered = std::iter::from_fn(|| fixtures.poll()).find_map(|event| match event {
        Event::Answer(got, answer) => Some((got, answer)),
        _ => None,
    });
    let Some((got, Answer::Storage(StorageAnswer::Written(Ok(Written::Wrote { wrote_as }))))) =
        answered
    else {
        panic!("the write was not answered as written");
    };
    assert_eq!(got, id);
    assert_eq!(wrote_as.as_str(), "signed.psbt");
    let listed = fixtures.written();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].1.as_str(), "signed.psbt");
}
