//! The strip's side of the demo tests: what Faraday Demo's host offers there.

use faraday_bip::keys::Network;
use faraday_core::keygen::{MIX_SOURCES, Source, Way, kstep};
use faraday_core::testkit::{self};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, demo};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

/// The passphrase of the empty vault in Files.
const VAULT_PHRASE: &str = "faraday";

/// An empty vault, in Files from the start.
fn vault_file() -> (String, Vec<u8>) {
    (
        "demo.kdbx".to_string(),
        crate::common::vault("Demo", VAULT_PHRASE, 9),
    )
}

/// The app with the test vault in Files and the clock checked: as the
/// demo has it (online, on testnet, the public seeds set up), or as the
/// stick has it.
fn app_on(demo: bool) -> Faraday {
    let mut app = crate::common::process(vec![vault_file()], Vec::new(), Vec::new());
    if demo {
        app.online = true;
        testkit::move_to(&mut app.session, Network::Testnet).unwrap();
        app.session.set_demo(faraday_core::testkit::demo_samples());
    }
    crate::common::keep_time(&mut app);
    tall(&mut app);
    app
}

/// A sample's fingerprint: `word` repeated `count` times.
fn sample_fp(word: &str, count: u8) -> [u8; 4] {
    let c = testkit::compute();
    let mut s = faraday_core::wallet::Session::default();
    s.keys
        .add_words(&c, &faraday_core::samples::repeated(word, count), "", None)
        .unwrap()
        .0
}

/// New key from `way` at `words` words, every entry made, on the Key
/// card. Returns the words made.
fn new_key(app: &mut Faraday, way: Way, words: u8) -> String {
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    app.press(Action::KWords(words));
    app.press(Action::KNext);
    app.press(Action::KWay(way.index()));
    if way == Way::Mix {
        let pos = |s: Source| MIX_SOURCES.iter().position(|m| *m == s).unwrap() as u8;
        app.press(Action::KMix(pos(Source::Dice)));
        app.press(Action::KMix(pos(Source::Coins)));
    }
    app.press(Action::KNext);
    for i in 0..3000 {
        let k = app.keygen().unwrap();
        if k.open == Some(kstep::KEY) {
            break;
        }
        let (have, need) = k.progress();
        if have < need {
            feed(app, i);
        } else {
            app.press(Action::KNext);
        }
    }
    let k = app.keygen().unwrap();
    assert_eq!(k.open, Some(kstep::KEY), "{way:?} at {words}: {:?}", k.note);
    k.phrase().expect("words made").to_string()
}

fn fingerprint(words: &str) -> [u8; 4] {
    let c = testkit::compute();
    let mut s = faraday_core::wallet::Session::default();
    s.keys.add_words_with(&c, words, "", "", None).unwrap().0
}

fn loaded(app: &Faraday) -> Vec<[u8; 4]> {
    app.session.keys.all().map(|k| k.fingerprint.0).collect()
}

/// Lock in, the words' quiz passed, Done.
fn finish(app: &mut Faraday) {
    use faraday_core::quiz::QuizState;
    app.press(Action::KLock);
    app.press(Action::KNext);
    app.press(Action::KNext);
    for _ in 0..30 {
        let Some(q) = app.keygen().and_then(|k| k.quiz.as_ref()) else {
            break;
        };
        if q.state() == QuizState::Passed {
            break;
        }
        let slot = q.correct_slot() as u8;
        app.press(Action::KQuiz(slot));
    }
    app.press(Action::KAdd);
}

// ---------------------------------------------------------------------
// Create a vault: Generate with dice and a typed passphrase.
// ---------------------------------------------------------------------

/// Thirty dice: six words of the long list.
const ROLLS: &str = "123451234512345123451234512345";

/// Create a vault from Vaults, its first passphrase from the dice or
/// typed, made and left on Unlock with it picked.
fn make_vault(app: &mut Faraday, typed: Option<&str>) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    assert_eq!(app.screen, Screen::CreateVault);
    match typed {
        None => {
            app.press(Action::Vault(V::Dice(0)));
            crate::common::type_text(app, ROLLS);
            app.press(Action::Vault(V::DiceUse));
        }
        Some(p) => {
            app.press(Action::Vault(V::CFocus(0, false)));
            crate::common::type_text(app, p);
            app.press(Action::Vault(V::CFocus(0, true)));
            crate::common::type_text(app, p);
        }
    }
    let _ = crate::common::texts(app);
    app.press(Action::Vault(V::CGo));
    crate::common::settle(app, 1);
    assert_eq!(app.screen, Screen::Unlock, "the vault was made");
}

/// Unlock with `phrase`. Returns whether the vault opened.
fn opens_with(app: &mut Faraday, phrase: &str) -> bool {
    let before = app.vaults().open.len();
    crate::common::unlock(app, phrase);
    app.vaults().open.len() > before
}

/// A display tall enough that an open card is drawn whole.
fn tall(app: &mut Faraday) {
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2400,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

// ---------------------------------------------------------------------
// Vanity, Lightning, silent payments.
// ---------------------------------------------------------------------

// ---------------------------------------------------------------------
// New key: every source runs; the next public seed is added.
// ---------------------------------------------------------------------

/// One entry for the source being entered.
fn feed(app: &mut Faraday, i: usize) {
    let Some(k) = app.keygen() else {
        return;
    };
    match k.active() {
        Some(Source::Dice) => app.press(Action::KRoll((i % 4) as u8 + 1)),
        Some(Source::Coins) => app.press(Action::KFlip((i * 5 + i / 3).is_multiple_of(2))),
        Some(Source::Hex) => app.press(Action::KHex((i * 7 % 16) as u8)),
        Some(Source::Cards) => {
            let c = (i % 52) as u8;
            app.press(Action::KRank(c % 13));
            app.press(Action::KSuit(c / 13));
        }
        Some(Source::Camera) => {
            let luma: Vec<u8> = (0..1024usize)
                .map(|p| ((p * 31 + i * 17 + p / 7) % 256) as u8)
                .collect();
            app.event(Event::CameraFrame {
                width: 32,
                height: 32,
                luma,
                chroma: None,
            });
            app.press(Action::KFrame);
        }
        Some(Source::Device) => {
            app.event(Event::Entropy(EntropyBytes::new([i as u8 ^ 0x3c; 32])));
            app.press(Action::KNext);
        }
        _ => app.press(Action::KNext),
    }
}

#[test]
fn in_the_demo_new_key_from_every_source_adds_a_sample_of_its_length_in_place_of_its_words() {
    let ways = [
        Way::DiceFlips,
        Way::Coins,
        Way::DiceWords,
        Way::DiceHashed,
        Way::DiceSixAsZero,
        Way::Hex,
        Way::Cards,
        Way::Mix,
        Way::Device,
    ];
    // Every source at 12 words, and every length from one source: the
    // sample put in place of the words does not depend on the other.
    let walks = ways.into_iter().map(|way| (way, 12u8, "twelve")).chain([
        (Way::Device, 15, "lucky"),
        (Way::Device, 18, "glad"),
        (Way::Device, 21, "brave"),
        (Way::Device, 24, "bacon"),
    ]);
    for (way, words, first) in walks {
        {
            let mut app = app_on(true);
            let made = new_key(&mut app, way, words);
            assert_eq!(made.split(' ').count(), usize::from(words));
            assert_ne!(made, faraday_core::samples::repeated(first, words));
            let says = format!("Lock in adds sample {first} \u{d7}{words}");
            assert!(
                crate::strips::strip(&mut app)
                    .iter()
                    .any(|t| t.contains(&says)),
                "{way:?} at {words}: the strip does not say so"
            );
            finish(&mut app);
            let have = loaded(&app);
            assert_eq!(have, vec![sample_fp(first, words)], "{way:?} at {words}");
            assert!(!have.contains(&fingerprint(&made)), "the key made was kept");
        }
    }
}

#[test]
fn a_sample_loaded_with_the_passphrase_counts_as_loaded_for_new_key() {
    let c = testkit::compute();
    let mut app = app_on(true);
    app.session
        .keys
        .add_words_with(
            &c,
            &faraday_core::samples::repeated("twelve", 12),
            faraday_core::samples::PASSPHRASE,
            "",
            None,
        )
        .unwrap();
    new_key(&mut app, Way::Device, 12);
    assert!(
        crate::strips::strip(&mut app)
            .iter()
            .any(|t| t.contains("Lock in adds sample ocean \u{d7}12"))
    );
}

#[test]
fn in_the_demo_a_vault_from_the_dice_shows_the_rolled_words_then_opens_with_faraday() {
    let rolled =
        faraday_core::dice_entry::dice_words(ROLLS, faraday_bip::diceware::List::Large).join("");
    let mut app = app_on(true);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::Dice(0)));
    crate::common::type_text(&mut app, ROLLS);
    // The rolled words are shown as they are rolled.
    let words = faraday_core::dice_entry::dice_words(ROLLS, faraday_bip::diceware::List::Large);
    assert!(crate::common::has_part(&mut app, words[0]));
    app.press(Action::Vault(V::DiceUse));
    assert!(
        crate::strips::strip(&mut app)
            .iter()
            .any(|t| t == "Practice puts faraday in place of these words")
    );
    // Both fields hold faraday, and the dice's strength is not measured.
    let c = app.vaults().create.as_ref().expect("the form");
    assert_eq!(c.phrases[0].0.text.as_str(), "faraday");
    assert_eq!(c.phrases[0].1.text.as_str(), "faraday");
    assert!(!crate::common::has_part(&mut app, "bits from dice"));
    app.press(Action::Vault(V::CGo));
    crate::common::settle(&mut app, 1);
    assert_eq!(app.screen, Screen::Unlock, "the vault was made");
    assert!(opens_with(&mut app, demo::PASSPHRASE));
    assert_ne!(rolled, demo::PASSPHRASE);

    // On the stick the rolled words are the passphrase.
    let mut app = app_on(false);
    make_vault(&mut app, None);
    assert!(opens_with(&mut app, &rolled));
}

#[test]
fn in_the_demo_use_faraday_puts_faraday_in_a_vaults_passphrase_and_its_repeat_and_it_opens_with_it()
{
    let mut app = app_on(true);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    let chip = Action::PassChip(demo::PassField::Phrase(0, false));
    assert!(
        crate::strips::strip_offers(&mut app, chip),
        "the pair offers faraday"
    );
    assert!(
        !crate::strips::strip_offers(&mut app, Action::PassChip(demo::PassField::Phrase(0, true))),
        "once, for both fields"
    );
    app.press(chip);
    assert!(
        !crate::strips::strip_offers(&mut app, chip),
        "gone once both fields hold it"
    );
    let c = app.vaults().create.as_ref().expect("the form");
    assert_eq!(c.phrases[0].0.text.as_str(), "faraday");
    assert_eq!(c.phrases[0].1.text.as_str(), "faraday");
    app.press(Action::Vault(V::CGo));
    crate::common::settle(&mut app, 1);
    assert_eq!(app.screen, Screen::Unlock, "the vault was made");
    assert!(opens_with(&mut app, demo::PASSPHRASE));
}
