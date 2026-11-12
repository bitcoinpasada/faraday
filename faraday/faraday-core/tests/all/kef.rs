//! A Krux encrypted code sealed and opened on the worker
//! (`docs/KEF.md` §6, §7.2, §14): what comes out opens to what went in,
//! a seal is never the same twice, and Cancel really stops the work.

use std::time::{Duration, Instant};

use faraday_bip::keys::Network;
use faraday_core::worker::kef::{KefId, KefStart};
use faraday_core::{Action, Faraday, testkit};
use faraday_keys::kef::{OpenError, unwrap};
use faraday_keys::{Keys, Reveal};
use faraday_shell_api::{App, EntropyBytes, Event};
use zeroize::Zeroizing;

const TWELVE: &str = "legal winner thank year wave sausage worth useful legal winner thank yellow";
const TWENTY_FOUR: &str = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";
const FIFTEEN: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon address";
const LINE: &str = "Working";
const COUNT: u32 = 10_000;

fn pass() -> Zeroizing<Vec<u8>> {
    Zeroizing::new(b"a passphrase".to_vec())
}

/// Gives the session `n` answers of fresh bytes, none the same as another
/// call's.
fn fresh(app: &mut Faraday, from: u8, n: u8) {
    for i in 0..n {
        app.event(Event::Entropy(EntropyBytes::new(
            [from.wrapping_add(i); 32],
        )));
    }
}

fn entropy_of(words: &str) -> Option<Zeroizing<Vec<u8>>> {
    let c = testkit::compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words_with(&c, words, "", "seed", None).unwrap();
    keys.kef_entropy(&c, 0, Reveal::KefCode)
        .map(|held| held.release())
}

fn seal(app: &mut Faraday, plaintext: &[u8]) -> Vec<u8> {
    app.kef_seal(
        pass(),
        Zeroizing::new(plaintext.to_vec()),
        20,
        KefId::Random(8),
        COUNT,
        LINE,
    )
    .expect("fresh bytes are here");
    testkit::settle(app);
    app.kef_take_sealed()
        .expect("an answer")
        .expect("sealed")
        .to_vec()
}

#[test]
fn a_12_and_a_24_word_seed_sealed_on_the_worker_open_to_the_same_entropy() {
    let mut app = testkit::started();
    fresh(&mut app, 0x10, 8);
    for words in [TWELVE, TWENTY_FOUR] {
        let entropy = entropy_of(words).expect("12 or 24 words");
        let code = seal(&mut app, &entropy);
        let envelope = unwrap(&code).unwrap();
        assert_eq!(envelope.count, COUNT);
        app.kef_open(pass(), envelope.clone(), LINE);
        testkit::settle(&mut app);
        let opened = app.kef_take_opened().unwrap().expect("opens");
        assert_eq!(&opened.plaintext[..], &entropy[..]);
        // The wrong passphrase gives nothing.
        app.kef_open(Zeroizing::new(b"another".to_vec()), envelope, LINE);
        testkit::settle(&mut app);
        assert_eq!(
            app.kef_take_opened().unwrap().err(),
            Some(OpenError::Failed)
        );
    }
}

#[test]
fn sealing_the_same_seed_twice_gives_different_codes() {
    let mut app = testkit::started();
    fresh(&mut app, 0x30, 8);
    let entropy = entropy_of(TWELVE).unwrap();
    let first = seal(&mut app, &entropy);
    let second = seal(&mut app, &entropy);
    assert_ne!(first, second);
    assert_ne!(unwrap(&first).unwrap().id, unwrap(&second).unwrap().id);
}

#[test]
fn a_seed_of_15_words_is_refused() {
    assert!(entropy_of(FIFTEEN).is_none());
}

#[test]
fn without_fresh_bytes_a_seal_does_not_start() {
    let mut app = testkit::started();
    let started = app.kef_seal(
        pass(),
        Zeroizing::new(vec![7; 16]),
        20,
        KefId::Random(8),
        COUNT,
        LINE,
    );
    assert_eq!(started, Err(KefStart::NoFreshBytes));
    assert!(app.busy().is_none());
}

#[test]
fn a_job_at_100_million_rounds_stops_on_cancel_and_the_next_job_runs() {
    let mut app = testkit::started();
    fresh(&mut app, 0x50, 8);
    let entropy = entropy_of(TWELVE).unwrap();
    let code = seal(&mut app, &entropy);
    let mut hostile = unwrap(&code).unwrap();
    hostile.count = 100_000_000;
    app.kef_open(pass(), hostile, LINE);
    let busy = *app.busy().expect("the job runs");
    assert_eq!(busy.of, 100_000_000);
    let began = Instant::now();
    app.press(Action::Cancel);
    assert!(app.busy().is_none());
    testkit::settle_stale(&mut app);
    // The next job runs at once: 100 million rounds would take far longer.
    let next = seal(&mut app, &entropy);
    assert!(unwrap(&next).is_ok());
    assert!(
        began.elapsed() < Duration::from_secs(20),
        "{:?}",
        began.elapsed()
    );
    // A stopped job leaves no answer behind.
    assert!(app.kef_take_opened().is_none());
}
