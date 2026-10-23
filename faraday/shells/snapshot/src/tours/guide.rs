//! The guide tour: the pictures in `docs/GUIDE.md` §2 and §3.

use crate::tour::{Tour, display, scroll_to};
use faraday_core::devices::DeviceCheck;
use faraday_core::keygen::{Way, kstep};
use faraday_core::{Action, Faraday, Medium, Screen, Sheet, StickInfo, StorageEvent};
use faraday_shell_api::{App, Event, Key};

/// EntropyLab's BitBox transcript for twelve words (`tools/reference/dice/
/// README.md`), the rolls the guide types: the last two are not taken.
const ROLLS: &str = "123411234122341233412344123415234126341231412342123413234124341235432141";

/// A fresh stick session as a person meets it: the stick it started from
/// still in, Secure Boot off at start, a laptop named for no maker; then
/// Home, the power sheet, Wallets, and New key made from the dice
/// transcript, with this device's randomness shown on the way, the words
/// and the quiz.
pub(crate) fn guide_tour(t: &mut Tour) -> Result<(), String> {
    let size = t.size();
    t.app = Faraday::new();
    t.app.medium = Medium::Stick;
    t.app.devices = DeviceCheck::machine("flags\t: fpu vme", "Example", "Example 14", "10");
    t.app.secure_boot = faraday_core::SecureBootAtStart::Off;
    t.app.event(display(size, t.dpi));
    t.app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        serial: Some(0x1234_5678),
        files: Vec::new(),
    }]));
    t.pump();
    for _ in 0..3 {
        t.tick();
    }
    t.shot("guide-home")?;
    t.press(Action::PowerAsk);
    t.shot("guide-power-sheet")?;
    t.app.event(Event::Key(Key::Escape));
    t.press(Action::Nav(Screen::WalletsStart));
    t.shot("guide-wallets")?;
    t.go_on = false;
    t.press(Action::KeyGen(None));
    t.shot("guide-pull-sheet")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    if t.app.sheet == Some(Sheet::Devices) {
        t.shot("guide-devices-sheet")?;
        t.press(Action::DevicesUnderstood);
    }
    t.go_on = true;
    t.shot("guide-newkey-start")?;
    t.press(Action::KNext);
    t.shot("guide-newkey-length")?;
    t.press(Action::KWords(12));
    t.shot("guide-newkey-randomness")?;
    new_key(t)
}

/// New key from Randomness on: this device's generator chosen and its
/// entry card, then the dice transcript, the words and the quiz.
fn new_key(t: &mut Tour) -> Result<(), String> {
    t.press(Action::KGroup(2));
    t.press(Action::KWay(Way::Device.index()));
    scroll_to(t, Action::KNext);
    t.shot("guide-newkey-randomness-device")?;
    t.press(Action::KNext);
    t.shot("guide-newkey-device-entry")?;
    t.press(Action::KStep(kstep::SOURCE));
    t.press(Action::KWay(Way::DiceWords.index()));
    t.shot("guide-newkey-randomness-dice")?;
    t.press(Action::KNext);
    t.shot("guide-newkey-rolls-empty")?;
    // A 5 in a word's first five places is not taken.
    t.press(Action::KRoll(5));
    t.shot("guide-newkey-rolls-five")?;
    for (i, c) in ROLLS.bytes().enumerate() {
        t.press(Action::KRoll(c - b'0'));
        if i == 23 {
            t.shot("guide-newkey-rolls-some")?;
        }
    }
    scroll_to(t, Action::KNext);
    t.shot("guide-newkey-rolls-done")?;
    t.press(Action::KNext);
    t.shot("guide-newkey-key")?;
    t.press(Action::KLock);
    t.shot("guide-newkey-locked")?;
    t.press(Action::KNext);
    t.shot("guide-newkey-words")?;
    t.press(Action::KShow);
    t.shot("guide-newkey-words-shown")?;
    t.press(Action::KShow);
    t.press(Action::KNext);
    t.shot("guide-newkey-quiz")?;
    let wrong = t
        .app
        .keygen()
        .and_then(|k| k.quiz.as_ref())
        .map(|q| (q.correct_slot() as u8 + 1) % 4)
        .ok_or("no quiz")?;
    t.press(Action::KQuiz(wrong));
    t.shot("guide-newkey-quiz-wrong")?;
    t.press(Action::KQuizRetry);
    super::default::pass_quiz(t);
    t.shot("guide-newkey-quiz-passed")?;
    t.press(Action::KAdd);
    t.shot("guide-newkey-loaded")
}
