//! The strip's side of the codex32_rest tests: what Faraday Demo's host offers there.

use faraday_bip::keys::Network;
use faraday_core::codex32::{C32, Card, SPLITS, Split, WAYS, Way};
use faraday_core::keygen::Start;
use faraday_core::samples::{SampleForm, SampleKey, SampleSize, Samples};
use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, Event};
use faraday_ui::keyboard::OskPress;

/// BIP 93's vector 2: shares A and C, D derived from them.
const SHARE_A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
const SHARE_C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
const SHARE_D: &str = "ms12namedll4f8jlh4e5vdvuldlfxu2jhdnlsm97xvenrxeg";
/// Vector 2's key: the fingerprint of its xprv.
const NAME_FP: [u8; 4] = [0xfa, 0xb6, 0x86, 0x8a];

fn c32(app: &mut Faraday, p: C32) {
    app.press(Action::Codex32(p));
}

fn flow(app: &Faraday) -> &faraday_core::codex32::Codex32Make {
    app.keygen()
        .and_then(|k| k.codex32.as_ref())
        .expect("Make codex32 shares open")
}

fn strings(app: &Faraday) -> Vec<String> {
    flow(app)
        .strings()
        .map(|s| s.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

/// Each string on the Strings card skipped, behind its caution: Strings
/// made open.
fn past_strings(app: &mut Faraday) {
    c32(app, C32::Next);
    assert_eq!(flow(app).open(), Some(Card::Strings));
    while flow(app).open() == Some(Card::Strings) {
        c32(app, C32::Skip);
        c32(app, C32::Skip);
    }
    assert_eq!(flow(app).open(), Some(Card::Made));
}

/// The demo's public set with its codex32 sample, BIP 93's vector 2, as
/// `faraday-samples` lists it (`docs/MODES.md` §7).
pub(crate) fn samples() -> &'static Samples {
    static SAMPLES: std::sync::OnceLock<Samples> = std::sync::OnceLock::new();
    SAMPLES.get_or_init(|| {
        let words = testkit::demo_samples();
        let mut keys = words.keys.to_vec();
        keys.push(SampleKey {
            name: "sample_codex32_shares",
            word: "",
            count: 0,
            form: SampleForm::Codex32,
            size: SampleSize::Shares(2),
            listed: true,
            parts: &[
                "MS12NAMEA320ZYXWVUTSRQPNMLKJHGFEDCAXRPP870HKKQRM",
                "MS12NAMECACDEFGHJKLMNPQRSTUVWXYZ023FTR2GDZMPY6PN",
            ],
            combined: "",
            plain: Some(NAME_FP),
            with_passphrase: None,
            node: None,
        });
        Samples {
            keys: Box::leak(keys.into_boxed_slice()),
            wallets: words.wallets,
            inputs: words.inputs,
        }
    })
}

/// The app as the demo has it, with the shell's samples.
fn demo() -> Faraday {
    let mut app = testkit::started();
    app.online = true;
    testkit::move_to(&mut app.session, Network::Testnet).unwrap();
    app.session.set_demo(samples());
    app.samples = Some(samples());
    // Tall enough that an open card is drawn whole.
    app.event(Event::Display(faraday_shell_api::DisplayInfo {
        width: 1366,
        height: 2400,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    app
}

/// Make codex32 shares open on its entry: 2 of 3 at 48 characters, the
/// identifier `d3m0`, `way` chosen.
fn entry(app: &mut Faraday, way: Way) {
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    app.press(Action::KeyGen(None));
    let _ = app.frame();
    let _ = app.frame();
    app.press(Action::KStart(Start::MakeCodex32.index()));
    let split = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
    c32(app, C32::Split(split as u8));
    c32(app, C32::Next);
    for ch in "d3m0".chars() {
        c32(app, C32::Key(OskPress::Char(ch)));
    }
    c32(app, C32::Next);
    let w = WAYS.iter().position(|w| *w == way).unwrap();
    c32(app, C32::Way(w as u8));
    c32(app, C32::Next);
    assert_eq!(
        flow(app).open(),
        Some(Card::Enter),
        "{:?}",
        flow(app).note()
    );
}

/// §12: in Practice the person rolls and watches the sheets; once the entry
/// is done the sample stands in place of the strings made, every later
/// card says so, no dice of theirs is read back, and Lock in loads the
/// sample's key. Use a sample fills the characters left.
#[test]
fn in_the_demo_the_sample_stands_in_place_of_the_strings_made() {
    let mut app = demo();
    entry(&mut app, Way::Dice);
    // One character rolled, the rest from Use a sample.
    for f in [4, 3, 4, 3, 4, 3, 4, 3, 4, 3] {
        c32(&mut app, C32::Key(OskPress::Char(char::from(b'0' + f))));
    }
    assert!(
        crate::strips::strip(&mut app)
            .iter()
            .any(|t| t == "Use a sample")
    );
    c32(&mut app, C32::Sample);
    let own = strings(&app);
    assert_eq!(own.len(), 3, "the entry finished");
    assert!(own.iter().all(|s| s.starts_with("ms12d3m0")));
    c32(&mut app, C32::Next);
    let m = flow(&app);
    assert_eq!(m.open(), Some(Card::Sheets));
    assert!(m.sample());
    assert_eq!(
        strings(&app),
        [SHARE_A, SHARE_C, SHARE_D],
        "the sample's strings"
    );
    assert!(
        m.tabs().iter().all(|t| !t.0.contains("Dice")),
        "no dice of the person's is read back"
    );
    assert!(
        crate::strips::strip(&mut app)
            .iter()
            .any(|t| t == "Practice puts sample NAME in place of these strings")
    );
    past_strings(&mut app);
    assert!(
        crate::strips::strip(&mut app)
            .iter()
            .any(|t| t == "Practice puts sample NAME in place of these strings")
    );
    c32(&mut app, C32::Lock);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys[0].fingerprint.0, NAME_FP);
}
