//! What collecting signatures asks for: the count still needed, and
//! "more" only once a signature is in.

#[test]
fn collecting_says_more_only_once_a_signature_is_in() {
    use faraday_core::wallet::collect_line;
    assert_eq!(collect_line(2, 0), "Collect 2 signatures");
    assert_eq!(collect_line(1, 1), "Collect 1 more signature");
    assert_eq!(collect_line(2, 1), "Collect 2 more signatures");
}

/// Savings' spend open on Review, gone through, with the Inbox's
/// `from-a.psbt` collected into it; test key 3 loaded. `answer` makes
/// that file from the spend.
fn reviewed_then_collected(
    answer: impl Fn(faraday_psbt::Psbt) -> Vec<u8>,
) -> faraday_core::Faraday {
    use crate::common::add_key;
    use faraday_core::{Action, StorageEvent, testkit};
    let k = testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .unwrap();
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), unsigned.to_bytes()),
            (
                faraday_text::Text::of("savings-wallet.txt"),
                k.descriptor.clone().into_bytes(),
            ),
            (faraday_text::Text::of("from-a.psbt"), answer(unsigned)),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |app: &faraday_core::Faraday, n: &str| {
        app.inbox().iter().position(|i| i.name == n).unwrap()
    };
    let w = at(&app, "savings-wallet.txt");
    testkit::load_wallet(&mut app, w);
    add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    testkit::steps_before_sign_done(&mut app);
    let n = at(&app, "from-a.psbt");
    app.press(Action::Collect(n));
    app
}

/// Review was gone through, then a Collect brought an output's key origin
/// that no loaded key can verify, a warning Review never showed: Sign Here
/// opens Review again, with the warning on it, and signs nothing.
#[test]
fn sign_here_opens_review_again_after_a_collect_adds_a_warning() {
    use faraday_core::Action;
    use faraday_core::wallet::step;
    let mut app = reviewed_then_collected(|unsigned| {
        let mut psbt = unsigned;
        let key = faraday_bip::bitcoin::secp256k1::PublicKey::from_slice(&[
            2, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87,
            0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b,
            0x16, 0xf8, 0x17, 0x98,
        ])
        .unwrap();
        psbt.inner_mut().outputs[0].bip32_derivation.insert(
            key,
            (
                faraday_bip::bitcoin::bip32::Fingerprint::from([9, 9, 9, 9]),
                "m/84'/0'/0'/1/0".parse().unwrap(),
            ),
        );
        psbt.to_bytes()
    });
    let s = app.spend().unwrap();
    let collected = s.spend.psbt.to_bytes();
    assert!(
        s.inspection
            .warnings
            .iter()
            .any(|w| w.kind == faraday_psbt::WarningKind::UnverifiedChange),
        "the Collect added no warning: {:?} {:?}",
        s.error,
        s.inspection.warnings
    );
    assert!(
        !s.done[step::TRANSACTION as usize],
        "Review reads done after the Collect"
    );
    app.press(Action::SignHere);
    let s = app.spend().unwrap();
    assert_eq!(s.spend.psbt.to_bytes(), collected, "signed without Review");
    assert_eq!(s.open, Some(step::TRANSACTION));
}

/// A Collect that only adds a cosigner's good signature changes nothing
/// Review showed: signing goes on without Review again.
#[test]
fn sign_goes_on_after_a_collect_that_only_adds_a_signature() {
    use faraday_bip::bitcoin::sighash::EcdsaSighashType;
    use faraday_core::Action;
    use faraday_core::wallet::step;
    let mut app =
        reviewed_then_collected(|_| crate::finish::signed_by_zebra_under(EcdsaSighashType::All));
    let s = app.spend().unwrap();
    let collected = s.spend.psbt.to_bytes();
    assert!(
        s.done[step::TRANSACTION as usize],
        "Review lapsed: {:?}",
        s.error
    );
    app.press(Action::SignHere);
    assert_ne!(
        app.spend().unwrap().spend.psbt.to_bytes(),
        collected,
        "Sign Here did not sign"
    );
}
