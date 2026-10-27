//! A key's calculations are blinded from the session's randomness
//! (`docs/learn/03-randomness.md`, `docs/KEYS.md` §2): a key's signing
//! context is randomized once when it is made, again from the session's
//! seed when it joins the session, or when the seed arrives if it joined
//! before, and again for every key whenever randomness the person adds
//! changes the seed. Blinding changes no result.

use crate::common::add_key;
use faraday_core::fresh::OwnSource;
use faraday_core::testkit;
use faraday_entropy::DiceRolls;
use faraday_shell_api::{App, EntropyBytes, Event};

fn blindings(app: &faraday_core::Faraday, i: usize) -> u32 {
    app.session.keys.master_for_tests(i).blindings()
}

fn add_dice(app: &mut faraday_core::Faraday) {
    let mut rolls = DiceRolls::new();
    for r in 1..=6 {
        assert!(rolls.push(r));
    }
    assert!(app.own_add(OwnSource::Dice, &rolls.commitment(), 6));
}

#[test]
fn a_key_joining_the_session_is_blinded_from_the_sessions_seed() {
    let mut app = testkit::started();
    app.event(Event::Entropy(EntropyBytes::new([0x11; 32])));
    add_key(&mut app, "bacon");
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(
        blindings(&app, 0),
        2,
        "blinded when made, and again from the session's seed"
    );
    add_key(&mut app, "zebra");
    assert_eq!(blindings(&app, 1), 2);
    assert_eq!(
        blindings(&app, 0),
        2,
        "another key joining blinds only itself"
    );
}

#[test]
fn every_key_is_blinded_again_when_the_person_adds_randomness() {
    let mut app = testkit::started();
    app.event(Event::Entropy(EntropyBytes::new([0x11; 32])));
    add_key(&mut app, "bacon");
    add_key(&mut app, "zebra");
    add_dice(&mut app);
    assert_eq!(blindings(&app, 0), 3);
    assert_eq!(blindings(&app, 1), 3);
}

#[test]
fn a_key_that_joined_before_the_seed_is_blinded_when_the_seed_arrives() {
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(blindings(&app, 0), 1, "no seed yet: blinded only when made");
    app.event(Event::Entropy(EntropyBytes::new([0x11; 32])));
    assert_eq!(blindings(&app, 0), 2);
}
