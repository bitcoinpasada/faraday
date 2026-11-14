//! The Review shows every character of each recipient's address, and on
//! a small panel Continue is offered once the end of the card is in view.

use crate::common::{add_key, has, offered, texts};
use faraday_core::strings::EN;
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, StorageEvent};
use faraday_psbt::Psbt;
use faraday_psbt::bitcoin::address::Address;
use faraday_psbt::bitcoin::{Amount, Network, ScriptBuf, WitnessProgram, WitnessVersion};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn kit() -> Kit {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap()
}

fn paying(version: WitnessVersion, program: &[u8]) -> (ScriptBuf, String) {
    let script = ScriptBuf::new_witness_program(&WitnessProgram::new(version, program).unwrap());
    let address = Address::from_script(&script, Network::Testnet).unwrap();
    (script, address.to_string())
}

/// The spending wallet's PSBT with its recipient's script replaced.
fn psbt_to(script: ScriptBuf) -> Psbt {
    let mut psbt = testkit::unsigned(&kit()).unwrap();
    psbt.inner_mut().unsigned_tx.output[0].script_pubkey = script;
    psbt
}

fn panel(app: &mut Faraday, width: u16, height: u16, dpi: u16) {
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// `psbt` open on Review on a panel of this size.
fn reviewing(psbt: &Psbt, size: (u16, u16, u16)) -> Faraday {
    let mut app = testkit::started();
    panel(&mut app, size.0, size.1, size.2);
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (
                faraday_text::Text::of("spending-wallet.txt"),
                kit().descriptor.into_bytes(),
            ),
            (faraday_text::Text::of("spend.psbt"), psbt.to_bytes()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    add_key(&mut app, "bacon");
    let at = app.inbox().iter().position(|i| i.name == "spend.psbt");
    app.press(Action::StartSpend(at.unwrap()));
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    app
}

/// Everything the frame draws with its spaces taken out: an address
/// grouped in fours and wrapped reads whole.
fn drawn(app: &mut Faraday) -> String {
    texts(app).concat().replace(' ', "")
}

const DESKTOP: (u16, u16, u16) = (1280, 800, 160);
const PI: (u16, u16, u16) = (480, 640, 286);

#[test]
fn the_review_shows_every_character_of_a_recipients_address() {
    for (version, len) in [(WitnessVersion::V0, 20), (WitnessVersion::V1, 32)] {
        let a = vec![0x11; len];
        let mut b = vec![0x11; len];
        // Differs in the middle only.
        b[len / 2] = 0x12;
        let (sa, addr_a) = paying(version, &a);
        let (sb, addr_b) = paying(version, &b);
        for size in [DESKTOP, PI] {
            let mut one = reviewing(&psbt_to(sa.clone()), size);
            let mut two = reviewing(&psbt_to(sb.clone()), size);
            let (shown_a, shown_b) = (drawn(&mut one), drawn(&mut two));
            assert!(
                shown_a.contains(&addr_a),
                "{size:?}: {addr_a} not in {shown_a}"
            );
            assert!(
                shown_b.contains(&addr_b),
                "{size:?}: {addr_b} not in {shown_b}"
            );
            assert_ne!(shown_a, shown_b);
        }
    }
}

#[test]
fn on_a_small_panel_continue_waits_for_the_end_of_a_long_review() {
    let (script, _) = paying(WitnessVersion::V0, &[0x22; 20]);
    let mut psbt = psbt_to(script);
    for k in 0..12 {
        let tx = &mut psbt.inner_mut().unsigned_tx;
        tx.output[1].value -= Amount::from_sat(10_000 + k);
        let mut out = tx.output[0].clone();
        out.value = Amount::from_sat(10_000 + k);
        tx.output.push(out);
        psbt.inner_mut().outputs.push(Default::default());
    }
    let mut app = reviewing(&psbt, PI);
    let next = Action::StepNext(step::TRANSACTION);
    assert!(!offered(&mut app, next));
    assert!(has(&mut app, EN.step_scroll_to_end));
    for _ in 0..60 {
        if offered(&mut app, next) {
            break;
        }
        let (w, h) = app.size();
        app.event(Event::Scroll {
            x: (w / 2.0) as u16,
            y: (h / 2.0) as u16,
            dy: 120,
        });
    }
    assert!(offered(&mut app, next), "Continue never came");
    assert!(!has(&mut app, EN.step_scroll_to_end));
}

#[test]
fn on_a_large_panel_continue_is_offered_at_once() {
    let (script, _) = paying(WitnessVersion::V0, &[0x22; 20]);
    let mut app = reviewing(&psbt_to(script), DESKTOP);
    assert!(offered(&mut app, Action::StepNext(step::TRANSACTION)));
}
