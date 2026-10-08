//! New key as SLIP-39 shares: the randomness is the master secret, dealt
//! as m of n shares; any m of them restore the key loaded, and each share
//! is quizzed before it is.

use faraday_core::keygen::{Way, kstep};
use faraday_core::{Action, Faraday, Screen};
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

fn ready() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app
}

/// A SLIP-39 key from 128 coin flips, dealt 2 of 3, up to its shares.
fn dealt() -> Faraday {
    let mut app = ready();
    app.press(Action::KeyGenSlip39);
    assert_eq!(app.screen, Screen::KeyGen);
    let k = app.keygen.as_ref().unwrap();
    assert!(k.slip39);
    assert_eq!((k.slip_m, k.slip_n, k.words), (2, 3, 20));
    app.press(Action::KWords(20));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip(i % 3 != 1));
    }
    app.press(Action::KNext);
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.open, Some(kstep::CHECK), "{:?}", k.note);
    app
}

#[test]
fn any_two_of_the_three_shares_restore_the_key() {
    let app = dealt();
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.shares.len(), 3);
    assert!(k.shares.iter().all(|s| s.word_count() == 20));
    let fp = k.fingerprint.unwrap();
    for pair in [[0usize, 1], [0, 2], [1, 2]] {
        let two = [k.shares[pair[0]].clone(), k.shares[pair[1]].clone()];
        let secret = osk_bip::slip39::recover(&two, b"").unwrap();
        let got = faraday_core::wallet::Session::default()
            .add_seed(secret.expose().as_bytes(), "")
            .unwrap();
        assert_eq!(got.0, fp, "shares {pair:?} restore another key");
    }
    let one = [k.shares[0].clone()];
    assert!(
        osk_bip::slip39::recover(&one, b"").is_err(),
        "one share is not enough"
    );
}

#[test]
fn the_key_loads_once_every_share_is_quizzed() {
    let mut app = dealt();
    app.press(Action::KNext); // check -> shares
    app.press(Action::KNext); // shares -> quiz
    let fp = app.keygen.as_ref().unwrap().fingerprint.unwrap();
    for _ in 0..200 {
        let k = app.keygen.as_ref().unwrap();
        if k.done[usize::from(kstep::QUIZ)] {
            break;
        }
        let slot = k.quiz.as_ref().unwrap().correct_slot() as u8;
        app.press(Action::KQuiz(slot));
    }
    assert!(app.keygen.as_ref().unwrap().done[usize::from(kstep::QUIZ)]);
    assert_eq!(
        app.keygen.as_ref().unwrap().quiz_share,
        2,
        "all three shares were quizzed"
    );
    app.press(Action::KAdd);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys[0].master.fingerprint().0, fp);
    assert!(
        app.session.keys[0].words.is_none(),
        "a SLIP-39 key has no BIP-39 words"
    );
}
