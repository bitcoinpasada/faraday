//! The site tour: the website tour's pictures, on the network the tour
//! runs on (Mainnet, as the guide), with the guide's key (3dca6805)
//! wherever a key shows. Five stops of Faraday's; the sixth is the
//! Starter's (`faraday-starter`'s `examples/snap.rs`, `--tour`).

use crate::tour::{Tour, scroll_to, type_text};
use crate::tours::guide::{files_in, key_session, kind_at, single_kit};
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::create::NewKind;
use faraday_core::ui::{Contrast, Mode, Theme, ThemeName};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent, cstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};

/// Costs what the parts it runs cost. Make codex32 shares on dice, its
/// rolls going in; Create a vault's passphrase from dice, its rolls
/// going in; Settings' Appearance in two themes; a 2-of-3's backup
/// board laid out; and a transaction's review with its warnings.
pub(crate) fn site_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    codex32_dice(t)?;
    vault_dice(t)?;
    appearance(t)?;
    let w = multisig(t)?;
    board(t, w)?;
    sign(c, t)
}

/// A camera's frame with nothing to read in it.
pub(crate) fn camera(t: &mut Tour) {
    t.app.event(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: (0..640u32 * 480).map(|i| ((i % 640) / 3) as u8).collect(),
        chroma: None,
    });
}

/// Make codex32 shares, 2 of 3 named `name`, on dice: share A's
/// characters (BIP 93's vector 2) rolled in, a picture after the 4th,
/// 10th, 16th and 22nd, each scrolled to the same place: the boxes, the
/// de-biasing tracks and the dice.
fn codex32_dice(t: &mut Tour) -> Result<(), String> {
    use crate::tours::codex32::{SHARES, roll};
    use faraday_core::codex32::{C32, OskPress, SPLITS, Split, WAYS, Way};
    use faraday_core::keygen::Start;
    crate::tours::screens::stick_session(t);
    let c = |t: &mut Tour, p: C32| t.press(Action::Codex32(p));
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    t.press(Action::KStart(Start::MakeCodex32.index()));
    let two = SPLITS
        .iter()
        .position(|s| *s == Split::TwoOfThree)
        .ok_or("no 2 of 3")?;
    c(t, C32::Split(two as u8));
    c(t, C32::Next);
    for ch in "name".chars() {
        c(t, C32::Key(OskPress::Char(ch)));
    }
    c(t, C32::Next);
    let dice = WAYS.iter().position(|w| *w == Way::Dice).ok_or("no dice")?;
    c(t, C32::Way(dice as u8));
    c(t, C32::Next);
    let a = &SHARES[0].as_bytes()[9..35];
    let mut frame = 0;
    for (i, &ch) in a.iter().enumerate() {
        roll(t, ch);
        if [4, 10, 16, 22].contains(&(i + 1)) {
            frame += 1;
            // What the screen moves by itself comes to rest first.
            for _ in 0..4 {
                t.tick();
            }
            hold_view(t, Action::Codex32(C32::View(0)), 213);
            t.shot_after(&format!("codex32-{frame}"), 0)?;
        }
    }
    Ok(())
}

/// The page scrolled until `action` is at `y`: the same view in every
/// frame of a loop, wherever the screen scrolled by itself.
fn hold_view(t: &mut Tour, action: Action, y: u16) {
    // A gesture over the page's right margin, outside the worksheet's
    // own scrolling, ended each time so the next starts afresh.
    let page = |t: &mut Tour, dy: i16| {
        let _ = t.app.frame();
        let (w, _) = t.size();
        let (x, y) = (w - 8, 150);
        t.app.event(Event::Scroll { x, y, dy });
        t.app.event(Event::ScrollEnd { x, y });
    };
    // From the top, where `action` is on screen.
    page(t, -30_000);
    for _ in 0..20 {
        t.app.settle();
        let _ = t.app.frame();
        let Some((_, at)) = t.app.where_offered(action) else {
            return;
        };
        let dy = i32::from(at) - i32::from(y);
        if dy == 0 {
            return;
        }
        page(t, dy.clamp(-2_000, 2_000) as i16);
    }
}

/// Rolls of five dice, as they land: six words of the long list.
const ROLLS: &str = "164532521346316254425163231564655213";

/// Create a vault with the guide's key loaded: the name Passwords, then
/// Generate with dice, a picture as the first, third, fifth and sixth
/// words come from their rolls.
fn vault_dice(t: &mut Tour) -> Result<(), String> {
    key_session(t);
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    t.press(Action::Vault(V::CName));
    type_text(t, "Passwords");
    t.press(Action::Vault(V::Dice(0)));
    let mut frame = 0;
    for n in 1..=6 {
        type_text(t, &ROLLS[(n - 1) * 5..n * 5]);
        if n == 1 {
            scroll_to(t, Action::Vault(V::DiceUse));
        }
        if [1, 3, 5, 6].contains(&n) {
            frame += 1;
            t.shot(&format!("vault-dice-{frame}"))?;
        }
    }
    Ok(())
}

/// Settings' Appearance, its theme chips, in Cobalt Light and in
/// Forest Dark.
fn appearance(t: &mut Tour) -> Result<(), String> {
    for (theme, name) in [
        (Theme::default(), "appearance"),
        (
            Theme::new(ThemeName::Forest, Mode::Dark, Contrast::Standard),
            "appearance-forest-dark",
        ),
    ] {
        t.press(Action::Theme(theme));
        t.press(Action::Nav(Screen::Settings));
        // Settings is a list of its sections; Appearance is the fifth.
        t.press(Action::SettingsSection(Some(4)));
        scroll_to(t, Action::Theme(theme));
        t.shot(name)?;
    }
    t.press(Action::Theme(Theme::default()));
    Ok(())
}

/// The guide's key and test keys 2 and 3 typed in, and a 2-of-3 made
/// over them. Returns the wallet.
fn multisig(t: &mut Tour) -> Result<usize, String> {
    key_session(t);
    for n in 1..3 {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    let multi = kind_at(NewKind::Multi)?;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CNext(cstep::QUORUM));
    t.press(Action::CVaults(false));
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint.0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.press(Action::CNext(cstep::KEYS));
    t.app
        .create()
        .and_then(|c| c.built)
        .ok_or_else(|| "the wallet was not made".to_string())
}

/// The 2-of-3's backup board, customized: a vault on a stick at Place 1
/// holding its seed and the description, and Place 2's seed with its
/// SeedQR.
fn board(t: &mut Tour, w: usize) -> Result<(), String> {
    t.press(Action::Backup(w));
    t.press(Action::Board(BP::Customize));
    t.press(Action::Board(BP::AddVault));
    t.press(Action::Board(BP::Chip(Chip::List(Thing::Vault(0)))));
    t.press(Action::Board(BP::Zone(Zone::Card(0))));
    t.press(Action::Board(BP::Menu(Chip::List(Thing::Seed(0)))));
    t.press(Action::Board(BP::MenuTo(Zone::Vault(0, 0, 0))));
    t.press(Action::Board(BP::Chip(Chip::List(Thing::Whole))));
    t.press(Action::Board(BP::Zone(Zone::Vault(0, 0, 0))));
    let seed = Chip::Placed(Zone::Paper(1), Thing::Seed(1));
    t.press(Action::Board(BP::Chip(seed)));
    t.press(Action::Board(BP::Form(Zone::Paper(1), Thing::Seed(1), 1)));
    t.press(Action::Board(BP::LetGo));
    t.shot("board")
}

/// Costs what reading a PSBT costs. A fresh session with the guide's
/// key, a single-key wallet made from it, and Sign a transaction on a
/// PSBT of that wallet whose fee is 0.012 BTC, brought into Files and
/// started there.
fn sign(c: &Compute, t: &mut Tour) -> Result<(), String> {
    key_session(t);
    t.press(Action::CreateWallet);
    t.press(Action::CKind(kind_at(NewKind::NativeSegwit)?));
    t.press(Action::CVaults(false));
    let fp = t.app.session.keys[0].fingerprint.0;
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CNext(cstep::KEYS));
    t.press(Action::Nav(Screen::Home));
    let kit = single_kit(c, t, 1_200_000)?;
    let psbt = faraday_core::testkit::unsigned(&kit)?.to_bytes();
    files_in(t, &[("payment.psbt", psbt.clone())], &[]);
    let at = t
        .app
        .inbox()
        .iter()
        .rposition(|i| i.bytes == psbt)
        .ok_or("the PSBT is not in Files")?;
    t.press(Action::StartSpend(at));
    t.shot("sign")
}
