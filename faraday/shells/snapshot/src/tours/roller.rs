//! The roller tour: Create a vault's Generate with dice with the dice
//! roller of Faraday Desktop (`docs/CODEX32.md` §5.6): at rest, with a
//! choice, mid-tumble, landed, the number typed, and rolled to the end
//! under Reduce motion.

use crate::tour::{Tour, scroll_to, type_text};
use faraday_core::vault::{RollerAct as Act, RollerPerClick as Per, VaultAction as V};
use faraday_core::{Action, StorageEvent};

pub(crate) fn roller_tour(t: &mut Tour) -> Result<(), String> {
    t.app.set_roller_offered(true);
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.tick();
    t.tick();
    t.press(Action::Vault(V::Create));
    t.press(Action::Vault(V::Dice(0)));
    let roll = Action::Vault(V::Roller(Act::Roll));
    scroll_to(t, roll);
    t.shot("roller-rest")?;
    t.press(Action::Vault(V::Roller(Act::Per(Per::Five))));
    t.press(roll);
    // Part-way through the first tumble, then once its value has landed.
    t.shot_after("roller-tumbling", 300)?;
    t.shot_after("roller-landed", 700)?;
    t.press(Action::Vault(V::Roller(Act::Stop)));
    t.press(Action::Vault(V::Roller(Act::Count)));
    type_text(t, "12");
    t.shot("roller-number")?;
    t.press(Action::Vault(V::Roller(Act::Per(Per::ToEnd))));
    t.press(Action::ReduceMotion(true));
    t.press(roll);
    t.tick();
    scroll_to(t, roll);
    t.shot("roller-to-the-end")?;
    t.press(Action::Vault(V::DiceUse));
    t.shot("roller-used")?;
    codex32_demo_only(t)
}

/// Make codex32 shares on twenty-faced dice: the computer's five dice in
/// the "Demo only" box beside the person's entry (`docs/CODEX32.md`
/// §5.5), at rest and after two passes rolled.
fn codex32_demo_only(t: &mut Tour) -> Result<(), String> {
    use faraday_core::codex32::{C32, DIE_SIZES, OskPress, WAYS, Way};
    use faraday_core::keygen::{Start, kstep};
    use faraday_core::testkit::five::Act as Five;
    let c = |t: &mut Tour, p: C32| t.press(Action::Codex32(p));
    t.press(Action::Nav(faraday_core::Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    t.press(Action::KStep(kstep::START));
    t.press(Action::KStart(Start::MakeCodex32.index()));
    c(t, C32::Next);
    for ch in "name".chars() {
        c(t, C32::Key(OskPress::Char(ch)));
    }
    c(t, C32::Next);
    let dice = WAYS.iter().position(|w| *w == Way::Dice).unwrap_or(0);
    c(t, C32::Way(dice as u8));
    let d20 = DIE_SIZES.iter().position(|&n| n == 20).unwrap_or(0);
    c(t, C32::Faces(d20 as u8));
    c(t, C32::Next);
    let roll = Action::Codex32(C32::Roller(Five::Roll));
    scroll_to(t, roll);
    t.shot("roller-codex32-demo-only")?;
    // One pass and the next: each die's two rolls side by side.
    t.press(Action::Codex32(C32::Roller(Five::Reach(
        faraday_core::testkit::five::Reach::Pass,
    ))));
    t.press(roll);
    t.tick();
    t.press(roll);
    t.tick();
    t.shot("roller-codex32-rolled")
}
