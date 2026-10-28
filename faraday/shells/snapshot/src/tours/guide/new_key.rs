//! The pictures of `docs/guide/walk/new-key.md`.

use crate::tour::{Tour, scroll_to};
use faraday_core::keygen::{Start, Way, kstep};
use faraday_core::{Action, Screen, Sheet, StorageEvent};

/// Wallets with nothing loaded, Add a key's Pull sheet with the boot
/// stick still in, Start, Length; then [`new_key`].
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::WalletsStart));
    t.shot_marking("3-1-wallets", Action::KeyGen(None))?;
    t.go_on = false;
    t.press(Action::KeyGen(None));
    t.shot("3-2-pull-the-stick")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    if t.app.sheet == Some(Sheet::Devices) {
        t.shot_marking("new-key-devices-sheet", Action::DevicesUnderstood)?;
        t.press(Action::DevicesUnderstood);
    }
    t.go_on = true;
    // Start opens closed on Make a new key with Length open (F103):
    // Change opens it, and Make a new key, pressed again, moves on.
    t.press(Action::KStep(kstep::START));
    t.shot_marking("3-3-start", Action::KStart(Start::Make.index()))?;
    t.press(Action::KStart(Start::Make.index()));
    t.shot_marking("3-4-length", Action::KWords(12))?;
    // A length moves on to Randomness by itself.
    t.press(Action::KWords(12));
    t.shot("new-key-randomness")?;
    new_key(t)
}

/// EntropyLab's BitBox transcript for twelve words (`tools/reference/dice/
/// README.md`), the rolls the guide types: the last two are not taken.
const ROLLS: &str = "123411234122341233412344123415234126341231412342123413234124341235432141";

/// New key from Randomness on: this device's generator chosen and its
/// entry card with dice of one's own on it, then the dice transcript,
/// the words and the quiz.
fn new_key(t: &mut Tour) -> Result<(), String> {
    // A way moves on to its entry card by itself (F103); Randomness,
    // opened again, shows the way chosen (F104).
    t.press(Action::KGroup(2));
    t.press(Action::KWay(Way::Device.index()));
    t.shot("3-6-generator")?;
    // Randomness of one's own, on the Generator card itself: Dice with
    // a few rolls in.
    {
        use faraday_core::fresh::OwnSource;
        use faraday_core::fresh::page::Act;
        t.press(Action::Own(Act::Source(OwnSource::Dice)));
        for f in [3, 1, 4, 1, 5, 2, 6, 5, 3, 5] {
            t.press(Action::Own(Act::Roll(f)));
        }
        t.shot("3-6-add-randomness")?;
    }
    t.press(Action::KStep(kstep::SOURCE));
    scroll_to(t, Action::KWay(Way::Device.index()));
    t.shot_marking(
        "3-5-randomness-device",
        Action::KWay(Way::DiceWords.index()),
    )?;
    t.press(Action::KWay(Way::DiceWords.index()));
    t.press(Action::KStep(kstep::SOURCE));
    t.shot_marking("3-5-randomness-dice", Action::KWay(Way::DiceWords.index()))?;
    t.press(Action::KWay(Way::DiceWords.index()));
    t.shot("new-key-rolls-empty")?;
    // A 5 in a word's first five places is not taken.
    t.press(Action::KRoll(5));
    t.shot("3-6-rolls-five")?;
    for (i, c) in ROLLS.bytes().enumerate() {
        t.press(Action::KRoll(c - b'0'));
        if i == 23 {
            t.shot("3-6-rolls")?;
        }
    }
    scroll_to(t, Action::KNext);
    t.shot_marking("3-6-rolls-done", Action::KNext)?;
    t.press(Action::KNext);
    t.shot_marking("3-7-key", Action::KLock)?;
    t.press(Action::KLock);
    t.shot("3-7-locked-in")?;
    t.shot_marking("new-key-words", Action::KShow)?;
    t.press(Action::KShow);
    t.shot_marking("3-8-words", Action::KNext)?;
    t.press(Action::KShow);
    t.press(Action::KNext);
    t.shot("3-9-quiz")?;
    let wrong = t
        .app
        .keygen()
        .and_then(|k| k.quiz.as_ref())
        .map(|q| (q.correct_slot() as u8 + 1) % 4)
        .ok_or("no quiz")?;
    t.press(Action::KQuiz(wrong));
    t.shot_marking("3-9-quiz-wrong", Action::KQuizRetry)?;
    t.press(Action::KQuizRetry);
    crate::tours::default::pass_quiz(t);
    t.shot_marking("3-9-quiz-passed", Action::KAdd)?;
    t.press(Action::KAdd);
    t.shot("3-10-after-done")
}
