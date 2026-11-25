//! The Sign flow's pane: the transaction's facts and the signatures box at
//! the Sign step (unscrolled, and scrolled to the pane's end) and at Finish,
//! for a first output of an ordinary address, an OP_RETURN and the longest
//! addresses.

use crate::tour::Tour;
use faraday_bip::bitcoin::ScriptBuf;
use faraday_core::testkit;
use faraday_core::testkit::Plain;
use faraday_core::wallet::step;
use faraday_core::{Action, StorageEvent};
use faraday_shell_api::{App, Event, Key};

fn recipient(len: usize) -> ScriptBuf {
    let mut script = vec![0x51u8];
    let program = if len == 62 { 32 } else { 40 };
    script.push(u8::try_from(program).unwrap());
    script.extend((0..program).map(|k| u8::try_from(k * 5 + 3).unwrap()));
    ScriptBuf::from_bytes(script)
}

pub(crate) fn spend_pane_tour(t: &mut Tour) -> Result<(), String> {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .ok_or("no kit")?;
    let mut op_return = vec![0x6a, 0x4c, 80];
    op_return.extend((0..80u8).map(|k| k.wrapping_mul(3)));
    let cases: [(&str, Option<ScriptBuf>); 4] = [
        ("plain", None),
        ("opreturn", Some(ScriptBuf::from_bytes(op_return))),
        ("c62", Some(recipient(62))),
        ("c74", Some(recipient(74))),
    ];
    let mut files = Vec::new();
    for (name, script) in &cases {
        let mut psbt = testkit::unsigned(&kit).map_err(|e| String::from(&*e))?;
        if let Some(s) = script {
            psbt.inner_mut().unsigned_tx.output[0].script_pubkey = s.clone();
        }
        files.push((format!("{name}.psbt"), psbt.to_bytes()));
    }
    files.push((
        "wallet.txt".to_string(),
        kit.descriptor.clone().into_bytes(),
    ));
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(files),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut t.app, cases.len());
    t.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        t.app.event(Event::Key(Key::Char(c)));
    }
    t.press(Action::EntryAdd);
    for (k, (name, _)) in cases.iter().enumerate() {
        t.press(Action::StartSpend(k));
        let steps: Vec<u8> = t.app.spend().map(|s| s.steps.to_vec()).unwrap_or_default();
        for n in steps.iter().copied().take_while(|&n| n != step::SIGN) {
            t.press(Action::StepNext(n));
        }
        t.shot(&format!("pane-{name}-sign"))?;
        // Scrolled to the pane's end.
        let _ = t.app.frame();
        let head = t
            .app
            .drawn_text_boxes()
            .plain()
            .into_iter()
            .find(|(s, _)| s == "Your choices")
            .map(|b| b.1);
        if let Some((hx, hy, _, _)) = head {
            let (x, y) = (
                u16::try_from(hx + 20).unwrap_or(0),
                u16::try_from(hy + 80).unwrap_or(0),
            );
            t.app.event(Event::Hover { x, y });
            t.app.event(Event::Scroll { x, y, dy: 4000 });
            t.app.event(Event::ScrollEnd { x, y });
            t.app.settle();
        }
        t.shot(&format!("pane-{name}-sign-end"))?;
        t.press(Action::SignHere);
        for n in steps.iter().copied().skip_while(|&n| n != step::SIGN) {
            t.press(Action::StepNext(n));
        }
        t.shot(&format!("pane-{name}-finish"))?;
        t.press(Action::Nav(faraday_core::Screen::WalletsStart));
    }
    Ok(())
}
