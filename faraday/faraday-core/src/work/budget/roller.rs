//! The dice roller (`docs/CODEX32.md` §5.6): a click, the tumble's ticks
//! and the frames between them do no costly work (`docs/SPEED.md` §8);
//! the values come from fresh randomness already asked for.

use faraday_shell_api::{App, EntropyBytes, Event};

use crate::fresh::roller::{Act, PerClick};
use crate::vault::VaultAction as V;
use crate::work::{KINDS, Work, since, snapshot};
use crate::{Action, testkit};

#[test]
fn rolling_ten_values_with_their_ticks_and_frames_does_no_costly_work() {
    let mut app = testkit::started();
    app.set_roller_offered(true);
    testkit::into_demo(&mut app);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::Dice(0)));
    app.press(Action::Vault(V::Roller(Act::Per(PerClick::Ten))));
    // The vault's own Argon2id measure runs on the first ticks: let it.
    for i in 1..=4u64 {
        app.event(Event::Tick {
            now_ms: app.now_ms() + i * 16,
        });
        let _ = app.frame();
    }
    let before = snapshot();
    app.press(Action::Vault(V::Roller(Act::Roll)));
    let start = app.now_ms();
    for i in 1..=700u64 {
        app.event(Event::Tick {
            now_ms: start + i * 16,
        });
        let _ = app.frame();
    }
    let work = since(&before);
    for k in KINDS {
        if matches!(k, Work::FullCanvas | Work::Blur) {
            continue;
        }
        assert!(work.of(k) == 0, "the roller did {k:?} {}", work.of(k));
    }
    let (_, rolls, _) = app.vaults().dice.as_ref().expect("the dice are open");
    assert_eq!(rolls.text.len(), 10, "all ten values landed");
}
