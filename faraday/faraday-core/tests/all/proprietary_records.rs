//! A PSBT that carries another program's proprietary records is signed
//! as any other: BIP 174 has a signer pass over the fields it does not
//! know.

use faraday_core::testkit;
use faraday_core::wallet::Origin;
use faraday_core::{Action, Screen, StorageEvent};

fn with_inbox(files: Vec<(String, Vec<u8>)>) -> faraday_core::Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

/// The Taproot test wallet's spend with its key loaded, with and without
/// another program's proprietary record on its input: signed either way.
#[test]
fn a_psbt_with_another_programs_proprietary_records_is_signed() {
    let c = testkit::compute();
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "taproot")
        .expect("the Taproot test wallet");
    for record in [false, true] {
        let (mut psbt, _) = testkit::spend_of(&kit).expect("its spend");
        if record {
            let key = faraday_psbt::bitcoin::psbt::raw::ProprietaryKey {
                prefix: b"another-wallet".to_vec(),
                subtype: 0,
                key: vec![2; 66],
            };
            psbt.inner_mut().inputs[0]
                .proprietary
                .insert(key, vec![0; 66]);
        }
        let before = psbt.to_bytes();
        let mut app = with_inbox(vec![("taproot-unsigned.psbt".to_string(), before.clone())]);
        testkit::move_to(&mut app.session, testkit::NET).expect("the test network");
        let (word, label) = testkit::TEST_SEEDS[0];
        app.session
            .keys
            .add_words(&c, &testkit::test_words(word), label, None)
            .expect("the key");
        app.session
            .add_wallet(&c, kit.name, &kit.descriptor, "Test wallet", Origin::Named)
            .expect("the wallet");
        app.press(Action::StartSpend(0));
        assert_eq!(app.screen, Screen::Spend);
        app.press(Action::SignHere);
        let s = app.spend().expect("the spend");
        assert!(s.complete, "{:?}", s.error);
    }
}
