//! What arrives and is shown rather than kept (`docs/COMPANION.md` §5,
//! §11): a web address is shown as text and nothing opens it; an address
//! is shown with its kind; other text is kept only when the person says
//! so; a BitLocker recovery key is shown masked and kept nowhere.

mod common;

use common::*;
use faraday_companion::store::Kind;
use faraday_companion::{Act, Page, Shown};
use osk_shell_api::Command;

#[test]
fn a_web_address_is_shown_as_text_and_nothing_opens_it() {
    for address in [
        faraday_learn::guide_address("windows"),
        "https://example.com/pay?to=me".to_string(),
    ] {
        let mut c = receiving();
        show(&mut c, &code_of(address.as_bytes()));
        assert_eq!(c.page, Page::Shown(Shown::Web(address.clone())));
        assert!(c.texts().contains(&address), "{:?}", c.texts());
        let (cmds, ours) = drain(&mut c);
        assert!(ours.is_empty(), "{ours:?}");
        // The camera goes off and the screen is drawn: nothing else.
        assert!(
            cmds.iter()
                .all(|k| matches!(k, Command::Draw | Command::CameraOff)),
            "{cmds:?}"
        );
        assert!(c.received().is_empty());
        // Nothing on the screen opens it either.
        assert!(!c.offers(Act::Open) && !c.offers(Act::Keep));
    }
}

#[test]
fn an_address_is_shown_with_its_kind_and_not_kept() {
    let address = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";
    let mut c = receiving();
    paste(&mut c, address);
    assert_eq!(
        c.page,
        Page::Shown(Shown::Address {
            text: address.into(),
            silent: false
        })
    );
    assert!(
        c.texts()
            .iter()
            .any(|t| t == faraday_strings::EN.companion_address_bitcoin)
    );
    assert!(c.received().is_empty());
}

#[test]
fn other_text_is_kept_only_when_the_person_says_so() {
    let text = "Meet at the north gate at noon.";
    let mut c = receiving();
    show(&mut c, &code_of(text.as_bytes()));
    assert_eq!(c.page, Page::Shown(Shown::Text(text.into())));
    assert!(c.received().is_empty());
    c.press(Act::Keep);
    let r = c.received();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, Kind::Text);
    assert_eq!(r[0].bytes, text.as_bytes());
}

#[test]
fn a_bitlocker_recovery_key_is_shown_masked_and_kept_nowhere() {
    let key = faraday_qr::bitlocker::KeyText {
        computer: "DESKTOP-4K2Q".into(),
        drive: "C:".into(),
        key: faraday_qr::bitlocker::RecoveryKey::parse(
            "205568-492019-108251-336446-650375-210331-584155-047520",
        )
        .expect("a key"),
    };
    let text = key.write().expect("the hub app's text");
    let mut c = receiving();
    let before = c.store_bytes();
    show(&mut c, &code_of(text.as_bytes()));
    assert_eq!(c.page, Page::BitLocker);
    assert_eq!(c.bitlocker(), Some(&key));
    let texts = c.texts();
    assert!(texts.iter().any(|t| t == "DESKTOP-4K2Q"), "{texts:?}");
    assert!(
        !texts.iter().any(|t| t.contains("205568")),
        "the key is shown unmasked: {texts:?}"
    );
    let (_, ours) = drain(&mut c);
    assert_eq!(kept_store(&ours), None);
    assert_eq!(c.store_bytes(), before);
    assert!(c.received().is_empty());
    // Pasted the same, and gone from memory once the screen is left.
    let mut c = receiving();
    paste(&mut c, &text);
    assert_eq!(c.page, Page::BitLocker);
    c.press(Act::Done);
    assert_eq!(c.bitlocker(), None);
    assert!(c.received().is_empty());
}
