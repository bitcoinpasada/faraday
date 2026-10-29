//! The New key tour, as Faraday Demo shows it (the line under a
//! field where a seed is typed on): Length with no count chosen, its
//! Other menu, the Generator with no source chosen, then dice of one's
//! own added on the card, Make the
//! key for a Make a new wallet slot with its passphrase fields and Show
//! words, and the word list searched by number.

use crate::tour::Tour;
use faraday_core::create::NewKind;
use faraday_core::cstep;
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
    t.press(Action::CNext(cstep::KIND));
    t.press(Action::KeyGen(Some(0)));
    if t.app.screen != Screen::KeyGen {
        return Err("Make a new wallet opened no New key".into());
    }
    // Start opens with nothing ticked; a choice opens Length.
    t.shot("keygen-start")?;
    t.press(Action::KStart(Start::Make.index()));
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
