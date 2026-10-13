//! What arrives and is shown rather than kept (`docs/COMPANION.md` §5,
//! §11): a web address is shown as text and nothing opens it; an address
//! is shown with its kind; other text is kept only when the person says
//! so; the address of one of the website's guides opens the
//! Companion's own copy of that page (§4.7).

mod common;

use common::*;
use faraday_companion::store::Kind;
use faraday_companion::{Act, Doc, Page, Shown};
use faraday_strings::EN;
use osk_shell_api::Command;

#[test]
fn a_web_address_is_shown_as_text_and_nothing_opens_it() {
    for address in [
        "https://example.com/pay?to=me".to_string(),
        // The website, but no guide the Companion has.
        faraday_learn::guide_address("no-such-guide"),
    ] {
        let mut c = receiving();
        show(&mut c, &code_of(address.as_bytes()));
        assert_eq!(c.page, Page::Shown(Shown::Web(address.clone())));
        assert!(shows(&mut c, &address), "{:?}", c.texts());
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
fn a_guide_address_opens_the_companions_own_page_and_asks_nothing_of_the_shell() {
    for id in ["windows", "maker-dell"] {
        let address = faraday_learn::guide_address(id);
        let mut c = receiving();
        show(&mut c, &code_of(address.as_bytes()));
        assert_eq!(c.page, Page::Doc(Doc::Own(id)), "{address}");
        let title = faraday_learn::known(id).title;
        assert!(shows(&mut c, title), "{address}: {:?}", c.texts());
        let (cmds, ours) = drain(&mut c);
        assert!(ours.is_empty(), "{ours:?}");
        assert!(
            cmds.iter()
                .all(|k| matches!(k, Command::Draw | Command::CameraOff)),
            "{cmds:?}"
        );
        assert!(c.received().is_empty());
        // Pasted, the same.
        let mut c = receiving();
        paste(&mut c, &address);
        assert_eq!(c.page, Page::Doc(Doc::Own(id)));
    }
}

#[test]
fn a_maker_page_shows_its_firmware_sketches_and_never_a_marker() {
    for m in faraday_learn::MAKERS.iter() {
        let mut c = receiving();
        paste(&mut c, &faraday_learn::guide_address(m.id));
        assert_eq!(c.page, Page::Doc(Doc::Own(m.id)));
        let texts = c.texts();
        assert!(
            !texts.iter().any(|t| t.contains("::sketch")),
            "{}: {texts:?}",
            m.id
        );
    }
    // Dell's first sketch is in the first screenful.
    let mut c = receiving();
    paste(&mut c, &faraday_learn::guide_address("maker-dell"));
    assert!(shows(&mut c, EN.ui_sketch_mock_up), "{:?}", c.texts());
}
