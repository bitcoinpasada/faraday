//! Check a multisig setup (`docs/FLOWS.md`): a descriptor, a policy or a
//! setup file read by Files' reader and shown as its quorum, checksum and
//! cosigners, with nothing added to the session.

use crate::common::has_part;
use faraday_bip::policy::WalletPolicy;
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::multisig_check::MsAction as M;
use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, Event};

fn kit(id: &str) -> String {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == id)
        .unwrap_or_else(|| panic!("{id}"))
        .descriptor
}

/// The check open with its scanner, `text` read on it.
fn scanned(text: &str) -> Faraday {
    let mut app = testkit::started();
    app.press(Action::MultisigCheck(M::Open));
    app.press(Action::MultisigCheck(M::Scan));
    app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
    let _ = app.frame();
    app
}

fn checksummed(descriptor: &str) -> String {
    let c = testkit::compute();
    WalletPolicy::parse_any(&c, descriptor)
        .unwrap()
        .to_descriptor_checksummed()
        .to_string()
}

#[test]
fn a_sorted_multisig_descriptor_shows_its_quorum_and_every_cosigner() {
    let d = kit("savings");
    let mut app = scanned(d.split('#').next().unwrap());
    assert!(has_part(&mut app, "2 of 3 · native SegWit multisig"));
    for n in 1..=3 {
        assert!(has_part(&mut app, &format!("Key {n}")));
    }
    // Each cosigner's fingerprint and origin path are shown.
    assert!(has_part(&mut app, "48h/1h/0h/2h"));
    assert!(has_part(&mut app, "Descriptor checksum"));
    assert!(has_part(&mut app, "worked out here, none given"));
    assert!(has_part(&mut app, "First receive address"));
    assert!(app.session.wallets.is_empty());
}

#[test]
fn a_checksum_that_holds_is_said_to_hold() {
    let mut app = scanned(&checksummed(&kit("savings")));
    assert!(has_part(&mut app, "holds"));
    assert!(!has_part(&mut app, "worked out here"));
}

#[test]
fn a_checksum_that_fails_is_refused_and_no_key_is_shown() {
    let good = checksummed(&kit("savings"));
    let (body, sum) = good.split_once('#').unwrap();
    let last = if sum.ends_with('q') { 'p' } else { 'q' };
    let bad = format!("{body}#{}{last}", &sum[..sum.len() - 1]);
    let mut app = scanned(&bad);
    assert!(has_part(&mut app, "The descriptor checksum does not hold"));
    assert!(!has_part(&mut app, "Key 1"));
}

/// Bitcoin Core's own documented multisig example and its checksum
/// (doc/descriptors.md): the checksum is read as given, not worked out here.
const CORE_EXAMPLE: &str = "wsh(multi(1,xpub661MyMwAqRbcFW31YEwpkMuc5THy2PSt5bDMsktWQcFF8syAmRUapSCGu8ED9W6oDMSgv6Zz8idoc4a6mr8BDzTJY47LJhkJ8UB7WEGuduB/1/0/*,xpub69H7F5d8KSRgmmdJg2KhpAK8SR3DjMwAdkxj3ZuxV27CprR9LgpeyGmXUbC6wb7ERfvrnKZjXoUmmDznezpbZb7ap6r1D3tgFxHmwMkQTPH/0/0/*))";

#[test]
fn a_checksum_from_bitcoin_core_is_accepted_and_a_changed_one_refused() {
    // Its keys have no origin, so it is refused, but not for its checksum.
    let mut app = scanned(&format!("{CORE_EXAMPLE}#t2zpj2eu"));
    assert!(!has_part(&mut app, "The descriptor checksum does not hold"));
    assert!(!has_part(&mut app, "Key 1"));
    let mut app = scanned(&format!("{CORE_EXAMPLE}#t2zpj2ev"));
    assert!(has_part(&mut app, "The descriptor checksum does not hold"));
}

#[test]
fn a_key_without_its_origin_is_refused() {
    let mut bare = String::new();
    let mut inside = false;
    for ch in kit("savings").split('#').next().unwrap().chars() {
        match ch {
            '[' => inside = true,
            ']' => inside = false,
            c if !inside => bare.push(c),
            _ => {}
        }
    }
    let mut app = scanned(&bare);
    assert!(has_part(&mut app, "key is not [fingerprint/path]xpub"));
    assert!(!has_part(&mut app, "Key 1"));
}

#[test]
fn a_wallet_of_one_key_is_not_a_setup() {
    let mut app = scanned(&kit("spending"));
    assert!(has_part(&mut app, "One key: not a multisig setup"));
}

#[test]
fn a_musig2_wallet_is_every_key_never_one_of_n() {
    let c = testkit::compute();
    let _ = c;
    let d = testkit::musig_policy().unwrap().to_descriptor_checksummed();
    let mut app = scanned(&d);
    assert!(has_part(&mut app, "3 of 3 · MuSig2"));
    assert!(!has_part(&mut app, "1 of 3"));
}

#[test]
fn a_script_tree_lists_its_ways_to_spend() {
    let mut app = scanned(&kit("tree"));
    assert!(has_part(&mut app, "Ways to spend"));
    assert!(has_part(&mut app, "Key path"));
}

#[test]
fn a_key_loaded_here_is_marked_yours_on_its_cosigner() {
    let mut app = testkit::started();
    let c = testkit::compute();
    app.session
        .keys
        .add_words(&c, &testkit::test_words("bacon"), "", None)
        .unwrap();
    app.press(Action::MultisigCheck(M::Open));
    app.press(Action::MultisigCheck(M::Scan));
    app.event(Event::Scanned {
        bytes: kit("savings").into_bytes(),
    });
    let _ = app.frame();
    assert!(has_part(&mut app, "Yours"));
}

#[test]
fn the_mark_is_the_loaded_key_after_the_way_back_in() {
    let mut app = testkit::started();
    let c = testkit::compute();
    app.session
        .keys
        .add_words(&c, &testkit::test_words("bacon"), "", None)
        .unwrap();
    app.press(Action::MultisigCheck(M::Open));
    app.press(Action::Nav(Screen::Home));
    app.session.wipe();
    app.press(Action::MultisigCheck(M::Open));
    app.press(Action::MultisigCheck(M::Scan));
    app.event(Event::Scanned {
        bytes: kit("savings").into_bytes(),
    });
    let _ = app.frame();
    assert!(!has_part(&mut app, "Yours"));
}

#[test]
fn a_file_of_files_is_checked_from_its_menu() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            "savings.txt",
            checksummed(&kit("savings")).into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.press(Action::File(FilePress::Menu(Row::In(0))));
    let lines = app
        .files_menu()
        .menu
        .as_ref()
        .expect("its menu")
        .lines
        .clone();
    let (_, check) = lines
        .iter()
        .find(|(l, _)| l == EN.files_check_setup)
        .unwrap_or_else(|| panic!("Check this setup is in {lines:?}"));
    let _ = app.frame();
    app.press(*check);
    assert_eq!(app.screen, Screen::MultisigCheck);
    assert!(has_part(&mut app, "2 of 3 · native SegWit multisig"));
    assert!(has_part(&mut app, "savings.txt"));
}
