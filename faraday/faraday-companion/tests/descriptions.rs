//! Wallet descriptions (`docs/COMPANION.md` §4.5, §7, §11): a kept
//! description's first address is the one its wallet publishes; a
//! description reaches the list from Received, from a scan on its own
//! screen and from a share, each after the phone's unlock and never
//! through Received's store; Rename keeps the name typed; the kept store
//! written and read back is the same.

mod common;

use common::*;
use faraday_companion::kept;
use faraday_companion::{Act, CompanionCommand, CompanionEvent, Page};
use faraday_ui::keyboard::OskPress;

/// A description kept from its own screen's scanner, by paste: the
/// screen it lands on.
fn kept_by_paste(text: &str) -> faraday_companion::Companion {
    let mut c = on_descriptions(&[]);
    c.press(Act::AddDescription);
    assert_eq!(c.page, Page::Receive);
    paste(&mut c, text);
    assert_eq!(c.page, Page::Description(0), "{text}");
    c
}

#[test]
fn a_kept_descriptions_first_address_is_the_one_its_wallet_publishes() {
    // BIP-84's account 0 of "abandon … about", as Jade writes it, and
    // its first receive address as BIP-84 publishes it.
    let mut c = kept_by_paste(&interop("sparrow/jade-keystore.txt"));
    assert!(
        screen_joined(&mut c).contains("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"),
        "{:?}",
        c.texts()
    );
    // The interop files that state their first address (docs/INTEROP.md):
    // Coldcard's account exports and BIP 129 records.
    let stated_coldcard = |text: &str| {
        let rest = &text[text.find("\"bip84\"").expect("bip84")..];
        let at = rest.find("\"first\"").expect("first") + "\"first\"".len();
        rest[at..]
            .split('"')
            .nth(1)
            .expect("an address")
            .to_string()
    };
    for path in [
        "bluewallet/coldcardmk4-generic.json",
        "bluewallet/coldcardQ.json",
        "bluewallet/nunchuk-export.json",
        "sparrow/cc-singlesig-keystore-1.json",
    ] {
        let text = interop(path);
        let mut c = kept_by_paste(&text);
        let first = stated_coldcard(&text);
        assert!(
            screen_joined(&mut c).contains(&first),
            "{path}: {first} not in {:?}",
            c.texts()
        );
    }
    for path in [
        "bluewallet/nunchuck-bsms.txt",
        "sparrow/bsms/multisig-1.bsms",
        "sparrow/bsms/multisig-3.bsms",
    ] {
        let text = interop(path);
        let first = text.lines().nth(3).expect("a first address").trim();
        let mut c = kept_by_paste(&text);
        assert!(
            screen_joined(&mut c).contains(first),
            "{path}: {first} not in {:?}",
            c.texts()
        );
    }
}

#[test]
fn a_description_reaches_wallet_descriptions_from_received_a_scan_and_a_share() {
    let text = description();

    // From Received: it arrives there as now, then the person keeps it.
    let mut c = receiving();
    show(&mut c, &code_of(text.as_bytes()));
    assert!(matches!(c.page, Page::Arrived { .. }));
    c.press(Act::Open);
    assert!(c.offers(Act::KeepInDescriptions));
    c.press(Act::KeepInDescriptions);
    let asked = unlock(&mut c, &[]);
    assert_eq!(c.page, Page::Description(0));
    let (_, ours) = drain(&mut c);
    let (bytes, n, k) = kept_kept(&ours).expect("the kept store written");
    assert_eq!((n, k), (1, 0));
    let store = kept::read(&bytes).expect("a store it reads");
    assert_eq!(store.descriptions[0].text, text);
    assert!(kept_store(&asked).is_some(), "Received kept it on arrival");

    // From a scan on its own screen: never through Received.
    let mut c = on_descriptions(&[]);
    c.press(Act::AddDescription);
    show(&mut c, &code_of(text.as_bytes()));
    assert_eq!(c.page, Page::Description(0));
    let (_, ours) = drain(&mut c);
    assert_eq!(kept_store(&ours), None);
    assert_eq!(kept_kept(&ours).map(|k| k.1), Some(1));
    assert!(c.received().is_empty());

    // Shared in from another app: the unlock is asked at that moment.
    let mut c = phone();
    c.companion_event(CompanionEvent::SharedIn {
        name: Some("spending.txt".into()),
        bytes: text.as_bytes().to_vec(),
    });
    unlock(&mut c, &[]);
    assert_eq!(c.page, Page::Description(0));
    let (_, ours) = drain(&mut c);
    assert_eq!(kept_store(&ours), None);
    let (bytes, ..) = kept_kept(&ours).expect("the kept store written");
    let store = kept::read(&bytes).expect("a store it reads");
    assert_eq!(store.descriptions[0].name, "spending");
    assert!(c.received().is_empty());
}

#[test]
fn a_description_scanned_anywhere_else_goes_to_received_as_before() {
    let mut c = receiving();
    show(&mut c, &code_of(description().as_bytes()));
    let (_, ours) = drain(&mut c);
    assert!(!asked_unlock(&ours));
    assert_eq!(kept_kept(&ours), None);
    assert_eq!(c.received().len(), 1);
}

#[test]
fn the_same_wallet_is_kept_once() {
    let text = interop("sparrow/descriptor-multipath.txt");
    let mut c = kept_by_paste(&text);
    c.press(Act::Back);
    c.press(Act::AddDescription);
    // The same wallet as a receive and a change descriptor.
    paste(&mut c, &interop("sparrow/descriptor-receive-change1.txt"));
    assert_eq!(c.page, Page::Description(0));
    assert_eq!(c.kept().map(|k| k.descriptions.len()), Some(1));
}

#[test]
fn renames_name_is_the_one_kept() {
    let mut c = kept_by_paste(&description());
    c.press(Act::Rename);
    assert!(matches!(c.page, Page::Rename(0)));
    // Typed on Faraday's own keyboard: every key pressed is one it shows.
    let press = |c: &mut faraday_companion::Companion, p: OskPress| {
        assert!(c.offers(Act::Osk(p)), "{p:?} not on the keyboard");
        c.press(Act::Osk(p));
    };
    for _ in 0..40 {
        press(&mut c, OskPress::Back);
    }
    press(&mut c, OskPress::Shift);
    for ch in "Savings".chars() {
        press(&mut c, OskPress::Char(ch));
    }
    press(&mut c, OskPress::Done);
    assert_eq!(c.page, Page::Description(0));
    let (_, ours) = drain(&mut c);
    let (bytes, ..) = kept_kept(&ours).expect("the kept store written");
    let store = kept::read(&bytes).expect("a store it reads");
    assert_eq!(store.descriptions[0].name, "Savings");
    assert!(shows(&mut c, "Savings"));
}

#[test]
fn the_kept_store_written_and_read_back_is_the_same() {
    let mut c = kept_by_paste(&description());
    c.press(Act::Back);
    c.press(Act::AddDescription);
    paste(&mut c, &interop("sparrow/jade-keystore.txt"));
    c.press(Act::Back);
    c.press(Act::Back);
    c.press(Act::Receive);
    paste(&mut c, &hub_key().write().expect("the hub app's text"));
    assert_eq!(c.page, Page::Key(0));
    let held = c.kept().cloned().expect("open");
    let (_, ours) = drain(&mut c);
    let (bytes, n, k) = kept_kept(&ours).expect("the kept store written");
    assert_eq!((n, k), (2, 1));

    // The next start: the counts at once, the store after the unlock.
    let mut again = phone();
    again.companion_event(CompanionEvent::KeptCounts {
        descriptions: n,
        keys: k,
    });
    assert_eq!(again.kept_counts(), (2, 1));
    assert!(again.kept().is_none());
    again.press(Act::Descriptions);
    unlock(&mut again, &bytes);
    assert_eq!(again.kept(), Some(&held));
    assert_eq!(kept::write(&held).to_vec(), bytes);
}

#[test]
fn share_hands_the_system_one_file_from_a_record_at_a_press() {
    let text = description();
    let mut c = kept_by_paste(&text);
    drain(&mut c);
    c.press(Act::Share);
    let (_, ours) = drain(&mut c);
    assert_eq!(
        ours,
        vec![CompanionCommand::Share {
            name: "Wallet 1.txt".into(),
            mime: "text/plain".into(),
            bytes: text.into_bytes(),
        }]
    );

    let mut c = receiving();
    show(
        &mut c,
        &code_of(osk_codec::ur::encode_psbt(&psbt()).as_bytes()),
    );
    c.press(Act::Open);
    drain(&mut c);
    c.press(Act::Share);
    let (_, ours) = drain(&mut c);
    let [CompanionCommand::Share { mime, bytes, .. }] = &ours[..] else {
        panic!("{ours:?}");
    };
    assert_eq!(mime, "application/octet-stream");
    assert_eq!(bytes, &psbt());
}
