//! The scanned Krux encrypted code tour (`docs/KEF.md` §15): its card,
//! the passphrase sheet and the one failure message.

use crate::tour::{Tour, scroll_to, type_text};
use faraday_core::board::{BoardPress as BP, Chip, Thing};
use faraday_core::kef_form::{IdKind, KefAct};
use faraday_core::kef_tool::{KefToolAction as K, Pick};
use faraday_core::{Action, Screen, qstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event, Key};

/// Krux's own AES-GCM code (`tools/vectors/kef/`).
const GCM: &str =
    "07746573742049441400000a4f52a1936c3e3271209e9d64bfb7766f5d5d8a4f908e86e738344c025d8fed54";

pub(crate) fn kef_tour(t: &mut Tour) -> Result<(), String> {
    let code: Vec<u8> = (0..GCM.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&GCM[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    t.press(Action::Nav(Screen::Home));
    t.press(Action::Scan);
    // The same code above what Krux writes: its count, three bytes after
    // the id and the version, reads 100 × 10,000 rounds.
    let mut high = code.clone();
    high[11] = 100;
    t.app.event(Event::Scanned { bytes: high });
    t.shot("kef-card-high")?;
    t.press(Action::ReadAgain);
    t.app.event(Event::Scanned { bytes: code });
    t.shot("kef-card")?;
    t.press(Action::ReadGo(1));
    t.shot("kef-passphrase")?;
    type_text(t, "wrong");
    t.app.event(Event::Key(Key::Enter));
    t.tick();
    t.tick();
    t.shot("kef-failed")
}

/// A 15-word seed's menu in the plan: its Krux code row is dimmed.
pub(crate) fn kef_menu_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Home));
    t.press(Action::Entry(None));
    type_text(t, &format!("{} address", vec!["abandon"; 14].join(" ")));
    t.press(Action::EntryAdd);
    let fp = t.app.session.keys.last().ok_or("no key")?.fingerprint.0;
    t.press(Action::KeyWallet(fp, 1));
    t.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    let w = t.app.session.wallets.len() - 1;
    t.press(Action::Backup(w));
    if t.app.backup().is_some_and(|b| b.q != Some(qstep::BOARD)) {
        t.press(Action::BQ(qstep::BOARD));
    }
    t.press(Action::Board(BP::Customize));
    if t.app.is_compact() {
        t.press(Action::Board(BP::OpenList));
    }
    t.press(Action::Board(BP::Menu(Chip::List(Thing::Seed(0)))));
    t.shot("kef-seed-menu-dimmed")
}

/// Costs what `key_text_of` and `add_wallet` cost. Tools' Krux code: Contents typed and from a loaded wallet, the first
/// step, Options, the work running, the Code card and Check my code
/// (`docs/KEF.md` §15.2).
pub(crate) fn kef_make_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    let form = |a| Action::KefTool(K::Form(a));
    t.press(Action::Entry(None));
    t.type_key(0);
    let text = crate::tour::key_text_of(
        c,
        &t.app.session.keys,
        0,
        faraday_core::create::NewKind::NativeSegwit,
    )?;
    t.app
        .session
        .add_wallet(
            c,
            "Spending",
            &format!("wpkh({text}/<0;1>/*)"),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    t.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "Krux code")
        .ok_or("no Krux code tile")?;
    t.press(Action::Catalog(tile as u8));
    t.shot("kef-make-contents")?;
    t.press(Action::KefTool(K::Pick(Pick::Wallet)));
    t.shot("kef-make-contents-wallet")?;
    t.press(Action::KefTool(K::Pick(Pick::Type)));
    type_text(t, "a note to keep with the backup");
    t.shot("kef-make-contents-typed")?;
    t.press(Action::KefTool(K::Next));
    t.shot("kef-make-first-step")?;
    type_text(t, "a passphrase");
    t.shot("kef-make-passphrase")?;
    t.press(form(KefAct::Options));
    t.shot("kef-make-options")?;
    t.press(form(KefAct::Rounds));
    for _ in 0..8 {
        t.app.event(Event::Key(Key::Backspace));
    }
    type_text(t, "10000");
    t.press(form(KefAct::Id(IdKind::Label)));
    type_text(t, "my label");
    t.shot("kef-make-options-typing")?;
    t.press(Action::Osk(faraday_core::OskPress::Hide));
    t.shot("kef-make-options-label")?;
    t.press(form(KefAct::Back));
    t.app.run_jobs_inline(false);
    faraday_core::testkit::hold_worker();
    faraday_core::testkit::press_on_worker(&mut t.app, form(KefAct::Make));
    t.shot("kef-make-making")?;
    faraday_core::testkit::release_worker();
    faraday_core::testkit::settle(&mut t.app);
    t.app.run_jobs_inline(true);
    t.tick();
    scroll_to(t, form(KefAct::Another));
    t.shot("kef-make-code")?;
    t.press(form(KefAct::Check));
    t.shot("kef-make-check")?;
    type_text(t, "a passphrase");
    t.press(form(KefAct::Compare));
    t.tick();
    scroll_to(t, form(KefAct::Another));
    t.shot("kef-make-checked")?;
    t.press(form(KefAct::Another));
    t.shot("kef-make-another")
}
