//! The sample transaction on the Pi's 2.8-inch panel, 480 × 640 pixels
//! at 286 dpi (268 dp wide), as Faraday Demo's Pi preview shows it:
//! nothing on the Spend tab or any step of the review is cut off at its
//! sides, at every place the page scrolls to.

use crate::psbt::with_savings;
use faraday_core::family::FamilyAction as F;
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, Event};

const W: u16 = 480;
const H: u16 = 640;

/// What the page drawn now has cut off at its sides, from its top to its
/// foot, each named with `what`.
fn cut_off(app: &mut Faraday, what: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut last = None;
    for _ in 0..60 {
        for p in app.drawn_past_sides() {
            let line = format!("{what}: {p}");
            if !found.contains(&line) {
                found.push(line);
            }
        }
        let shown = Some(app.drawn_text_boxes());
        if shown == last {
            break;
        }
        last = shown;
        app.event(Event::Scroll {
            x: W / 2,
            y: H / 2,
            dy: 60,
        });
        app.settle();
    }
    for _ in 0..60 {
        app.event(Event::Scroll {
            x: W / 2,
            y: H / 2,
            dy: -60,
        });
    }
    app.settle();
    found
}

#[test]
fn nothing_of_the_sample_spend_is_cut_off_on_the_pi_panel() {
    let mut app = with_savings();
    crate::common::show(&mut app, W, H, 286);
    let mut found = Vec::new();
    app.press(Action::Nav(Screen::Wallets));
    found.extend(cut_off(&mut app, "Wallets"));
    app.press(Action::OpenWallet(0));
    found.extend(cut_off(&mut app, "Savings"));
    app.press(Action::Nav(Screen::Family));
    found.extend(cut_off(&mut app, "Spend tab"));
    app.press(Action::Family(F::SpendFrom(0)));
    found.extend(cut_off(&mut app, "Spend from Savings"));
    app.press(Action::SamplePsbt);
    for s in [
        step::TRANSACTION,
        step::TXID,
        step::SIGNERS,
        step::SIGN,
        step::WALLET,
        step::CHECK,
    ] {
        app.press(Action::Step(s));
        found.extend(cut_off(&mut app, &format!("sample spend, step {s}")));
    }
    app.press(Action::SignHere);
    for s in [step::COLLECT, step::FINISH] {
        app.press(Action::Step(s));
        found.extend(cut_off(&mut app, &format!("sample spend signed, step {s}")));
    }
    assert!(
        found.is_empty(),
        "cut off on the Pi's panel:\n{}",
        found.join("\n")
    );
}
