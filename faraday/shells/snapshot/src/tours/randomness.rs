//! The randomness tour (`docs/OWN-RANDOMNESS.md` §2.2): Settings'
//! Randomness row, Add randomness as it opens with nothing chosen, on
//! Camera with a picture taken, Dice with a batch entered, Cards with a
//! rank picked, Pointer and keys, and the row again with what was added;
//! then the Randomness row at a press that draws the system's randomness
//! (§8): New key's SLIP-39 shares on their Enter step, with the ⋮ on the
//! Pi's panel, and Add randomness opened from there.

use crate::tour::Tour;
use faraday_core::fresh::OwnSource;
use faraday_core::fresh::page::Act;
use faraday_core::keygen::Way;
use faraday_core::{Action, Screen, StorageEvent};
use faraday_shell_api::{App, EntropyBytes, Event};

pub(crate) fn randomness_tour(t: &mut Tour) -> Result<(), String> {
    let own = |a| Action::Own(a);
    // As the desktop runs it: a pointer and keys.
    t.app.set_pointer_keys_offered(true);
    t.app.event(Event::EntropySource("getrandom(2)"));
    t.app.event(Event::Entropy(EntropyBytes::new([0x11; 32])));
    t.app.storage(StorageEvent::Cameras(vec![(
        "cam0".to_string(),
        "Camera".to_string(),
    )]));
    t.press(Action::Nav(Screen::Settings));
    t.shot("randomness-settings")?;
    t.press(own(Act::Open));
    t.shot("randomness-open")?;
    t.press(own(Act::Source(OwnSource::Camera)));
    t.shot("randomness-camera")?;
    // lint-tokens: sample data
    let luma: Vec<u8> = (0..=255u8).cycle().take(64 * 64).collect();
    // lint-tokens: end sample data
    t.app.event(Event::CameraFrame {
        width: 64,
        height: 64,
        luma,
        chroma: None,
    });
    t.press(own(Act::Frame));
    t.shot("randomness-camera-taken")?;
    t.press(own(Act::Source(OwnSource::Dice)));
    for f in [3, 1, 4, 1, 5, 2, 6, 5, 3, 5] {
        t.press(own(Act::Roll(f)));
    }
    t.shot("randomness-dice")?;
    t.press(own(Act::Add));
    t.press(own(Act::Source(OwnSource::Cards)));
    t.press(own(Act::Rank(0)));
    t.press(own(Act::Suit(1)));
    t.press(own(Act::Rank(11)));
    t.shot("randomness-cards")?;
    t.press(own(Act::Source(OwnSource::PointerKeys)));
    t.shot("randomness-pointer-keys")?;
    t.press(Action::Back);
    t.shot("randomness-settings-added")?;
    // Dealing SLIP-39 shares draws the system's randomness: the row above
    // Next (`docs/OWN-RANDOMNESS.md` §8.1 row 2).
    t.press(Action::KeyGenSlip39);
    t.press(Action::KWords(20));
    t.press(Action::KWay(Way::Coins.index()));
    t.press(Action::KNext);
    for i in 0..128 {
        t.press(Action::KFlip(i % 3 != 1));
    }
    t.shot("randomness-at-a-press")?;
    t.press(own(Act::Open));
    t.shot("randomness-from-a-press")
}
