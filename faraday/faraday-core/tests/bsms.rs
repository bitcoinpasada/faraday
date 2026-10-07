//! BIP 129 both ways: the BIP's own records (its NO_ENCRYPTION session)
//! load as a wallet and as a cosigner's key, a record whose signature or
//! first address is wrong is refused, and the key records and descriptor
//! records Faraday writes read back as the same key and wallet.

use faraday_core::wallet::{FileKind, classify};
use faraday_core::{Action, Faraday, StorageEvent};
use osk_shell_api::App;

const SIGNER_1: &str = "BSMS 1.0
00
[1cf0bf7e/48'/0'/0'/2']xpub6FL8FhxNNUVnG64YurPd16AfGyvFLhh7S2uSsDqR3Qfcm6o9jtcMYwh6DvmcBF9qozxNQmTCVvWtxLpKTnhVLN3Pgnu2D3pAoXYFgVyd8Yz
Signer 1 key
IB7v+qi1b+Xrwm/3bF+Rjl8QbIJ/FMQ40kUsOOQo1SqUWn5QlFWbBD8BKPRetfo1L1N7DmYjVscZNsmMrqRJGWw=";

const DESCRIPTOR: &str = "BSMS 1.0
wsh(sortedmulti(2,[1cf0bf7e/48'/0'/0'/2']xpub6FL8FhxNNUVnG64YurPd16AfGyvFLhh7S2uSsDqR3Qfcm6o9jtcMYwh6DvmcBF9qozxNQmTCVvWtxLpKTnhVLN3Pgnu2D3pAoXYFgVyd8Yz/**,[4fc1dd4a/48'/0'/0'/2']xpub6EebMbEps7ZcV3FYEnddRsvrFWDrt2tiPmCeM7pPXQEmphvq9ZfJ1LWFUDjf3vxCeBuPrfyGrMazWUsYsetrnHatQZVLJH7LsgCjtMqdzgj/**))
/0/*,/1/*
bc1qrgc6p3kylfztu06ysl752gwwuekhvtfh9vr7zg43jvu60mutamcsv948ej";

fn with_inbox(files: Vec<(&str, String)>) -> Faraday {
    let mut app = Faraday::new();
    app.storage(StorageEvent::Restored {
        inbox: files
            .into_iter()
            .map(|(n, t)| (n.to_string(), t.into_bytes()))
            .collect(),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

#[test]
fn the_bips_descriptor_record_loads_as_its_wallet() {
    let mut app = with_inbox(vec![("savings-bsms.txt", DESCRIPTOR.to_string())]);
    assert_eq!(app.inbox[0].kind, FileKind::Wallet);
    app.press(Action::LoadWallet(0));
    assert_eq!(app.session.wallets.len(), 1);
    let w = &app.session.wallets[0];
    assert_eq!(
        app.session.address(w, false, 0),
        "bc1qrgc6p3kylfztu06ysl752gwwuekhvtfh9vr7zg43jvu60mutamcsv948ej"
    );
    // A record whose first address is not the wallet's is not a wallet.
    let wrong = DESCRIPTOR.replace(
        "bc1qrgc6p3kylfztu06ysl752gwwuekhvtfh9vr7zg43jvu60mutamcsv948ej",
        "bc1qhs4u273g4azq7kqqpe6vh5wfhasfmrq7nheyzsnq77humd7rwtkqagvakf",
    );
    assert_ne!(classify("x.txt", wrong.as_bytes()), FileKind::Wallet);
}

#[test]
fn the_bips_key_record_is_a_cosigners_key_only_with_its_signature() {
    assert_eq!(classify("k.txt", SIGNER_1.as_bytes()), FileKind::Key);
    let forged = SIGNER_1.replace("Signer 1 key", "Signer 9 key");
    assert_ne!(classify("k.txt", forged.as_bytes()), FileKind::Key);
}

#[test]
fn faradays_own_records_read_back() {
    let mut app = Faraday::new();
    app.press(Action::Entry(None));
    for c in faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[0].0).chars() {
        app.event(osk_shell_api::Event::Key(osk_shell_api::Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::CreateWallet);
    app.press(Action::CKind(4));
    app.press(Action::CNext(0));
    app.press(Action::CNext(1));
    let fp = app.session.keys[0].master.fingerprint().0;
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CKeyBsms(0));
    let record = app
        .outbox
        .iter()
        .find(|i| i.name.ends_with("-bsms.txt"))
        .expect("no key record");
    assert_eq!(record.kind, FileKind::Key);
    let key = faraday_core::create::read_key(std::str::from_utf8(&record.bytes).unwrap()).unwrap();
    let plain = faraday_core::create::NewKind::Multi
        .key_text(&app.session.keys[0].master)
        .unwrap();
    assert_eq!(key.replace('\'', "h"), plain.replace('\'', "h"));
}
