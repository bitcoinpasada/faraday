//! The New key tour, as Faraday Demo shows it (the line under a
//! field where a seed is typed on): Length with no count chosen, its
//! Other menu, the Generator with no source chosen, then dice of one's
//! own added on the card, Make the
//! key for a Make a new wallet slot with its passphrase fields and Show
//! words, and the word list searched by number.

use crate::tour::Tour;
use faraday_core::create::NewKind;
use faraday_core::fresh::OwnSource;
use faraday_core::fresh::page::Act;
use faraday_core::keygen::{Start, Way};
use faraday_core::wordlist::WordListAction as WL;
use faraday_core::{Action, Screen};
use faraday_shell_api::{App, EntropyBytes, Event, Key};

pub(crate) fn keygen_tour(t: &mut Tour) -> Result<(), String> {
    t.app.seed_line = true;
    t.app.event(Event::EntropySource("getrandom(2)"));
    t.app.event(Event::Entropy(EntropyBytes::new([0x11; 32])));
    t.press(Action::CreateWallet);
    let at = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::NativeSegwit)
        .unwrap_or(0) as u8;
    t.press(Action::CKind(at));
    t.press(Action::CBackupForm(
        faraday_core::create::BackupForm::Words.index(),
    ));
    t.press(Action::KeyGen(Some(0)));
    if t.app.screen != Screen::KeyGen {
        return Err("Make a new wallet opened no New key".into());
    }
    // Start opens with nothing ticked; a choice opens Length.
    t.shot("keygen-start")?;
    t.press(Action::KStart(
        Start::Make(faraday_core::forms::Form::Words).index(),
    ));
    t.shot("keygen-length")?;
    t.press(Action::KOtherLengths);
    t.shot("keygen-length-other")?;
    // 18 words chosen there moves on to Randomness.
    t.press(Action::KWords(18));
    t.shot("keygen-source")?;
    t.press(Action::KWay(Way::Device.index()));
    t.app.event(Event::Entropy(EntropyBytes::new([0x6c; 32])));
    t.shot("keygen-generator")?;
    t.press(Action::Own(Act::Source(OwnSource::Dice)));
    for f in [3, 1, 4, 1, 5] {
        t.press(Action::Own(Act::Roll(f)));
    }
    t.shot("keygen-generator-dice")?;
    t.press(Action::Own(Act::Add));
    t.app.event(Event::Entropy(EntropyBytes::new([0x6d; 32])));
    t.shot("keygen-generator-added")?;
    t.press(Action::KNext);
    t.shot("keygen-key")?;
    // Down to the passphrase fields, the line under them and Show words.
    for _ in 0..4 {
        let _ = t.app.frame();
        t.app.event(Event::Scroll {
            x: 100,
            y: 300,
            dy: 600,
        });
    }
    t.shot("keygen-key-foot")?;
    t.press(Action::WordList(WL::Open(0, None)));
    for c in "101".chars() {
        t.app.event(Event::Key(Key::Char(c)));
    }
    t.shot("wordlist-find-101")
}

/// New key from dice words (six rolls a word) as Words so far shows
/// them: part way through the first word, with seven words made, and
/// with the Guide open over that (the pane, or a sheet or page where
/// the window holds no pane).
pub(crate) fn dice_words_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::KeyGen(None));
    t.press(Action::KStart(
        Start::Make(faraday_core::forms::Form::Words).index(),
    ));
    t.press(Action::KWords(12));
    t.press(Action::KWay(Way::DiceWords.index()));
    t.press(Action::KNext);
    t.press(Action::KTyping(false));
    for f in [3, 1, 4] {
        t.press(Action::KRoll(f));
    }
    t.shot("dicewords-mid-word")?;
    for i in 3..42u8 {
        t.press(Action::KRoll(i % 6 + 1));
    }
    for _ in 0..4 {
        t.tick();
    }
    t.shot("dicewords-some-words")?;
    // On the Pi's panel Words so far is a button and a page of its own,
    // below the pad: the card is scrolled to it.
    for _ in 0..6 {
        t.app.settle();
        let _ = t.app.frame();
        if t.app.where_offered(Action::KWordsPage(true)).is_some() {
            break;
        }
        crate::tour::scroll(t, 120);
    }
    if t.app.where_offered(Action::KWordsPage(true)).is_some() {
        crate::tour::scroll_to(t, Action::KWordsPage(true));
        crate::tour::scroll(t, 120);
        t.shot("dicewords-button")?;
        t.press(Action::KWordsPage(true));
        t.shot("dicewords-page")?;
        t.press(Action::KWordsPage(false));
    }
    t.press(Action::Guide);
    for _ in 0..4 {
        t.tick();
    }
    t.shot("dicewords-guide")
}
