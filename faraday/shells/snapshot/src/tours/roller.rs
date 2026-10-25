//! The roller tour: Create a vault's Generate with dice with the dice
//! roller of Faraday Desktop (`docs/CODEX32.md` §5.6): at rest, with a
//! choice, mid-tumble, landed, the number typed, and rolled to the end
//! under Reduce motion.

use crate::tour::{Tour, scroll_to, type_text};
use faraday_core::vault::{RollerAct as Act, RollerPerClick as Per, VaultAction as V};
use faraday_core::{Action, StorageEvent};

pub(crate) fn roller_tour(t: &mut Tour) -> Result<(), String> {
    t.app.set_roller_offered(true);
    // The roller is Demo's alone (`docs/DEMO.md` §14).
    faraday_core::testkit::into_demo(&mut t.app);
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
/// §5.5), at rest, then the step-through a step at a time: a die's first
/// roll, a second roll and its bit, the character into its box, a sum of
/// the checksum worksheet and a table row, and a frame while the
/// worksheet's view moves to follow the work.
fn codex32_demo_only(t: &mut Tour) -> Result<(), String> {
    use faraday_core::codex32::{C32, DIE_SIZES, OskPress, Step, WAYS, WalkAct, Way};
    use faraday_core::keygen::{Start, kstep};
    let c = |t: &mut Tour, p: C32| t.press(Action::Codex32(p));
    // The steps are shown as they are, with motion.
    t.press(Action::ReduceMotion(false));
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
    // The card holds its place: at the right below the top strip, or a
    // strip at the window's foot; the page is at its top.
    crate::tour::scroll(t, -3000);
    t.shot("roller-codex32-demo-only")?;
    // The step shown, as the walk holds it.
    let step = |t: &Tour| {
        t.app
            .keygen()
            .and_then(|k| k.codex32.as_ref())
            .and_then(|m| m.walk().shown().map(|(_, _, s)| s))
    };
    // Next until `want` says the step shown is the one, at most 80.
    let to = |t: &mut Tour, want: &dyn Fn(Step) -> bool| {
        for _ in 0..80 {
            c(t, C32::Walk(WalkAct::Next));
            t.tick();
            if step(t).is_some_and(want) {
                return;
            }
        }
    };
    to(t, &|s| matches!(s, Step::First { .. }));
    t.shot("roller-codex32-step-die")?;
    to(t, &|s| matches!(s, Step::Second { .. }));
    t.shot("roller-codex32-step-bit")?;
    to(t, &|s| s == Step::Char);
    t.shot("roller-codex32-step-character")?;
    // On a small panel the checksum worksheet is its own tile.
    if t.size().0 < 600 {
        c(t, C32::Panel(1));
    }
    // The page scrolled to the worksheet's cells the step marks.
    let marked = |t: &mut Tour| {
        let cell = t
            .app
            .keygen()
            .and_then(|k| k.codex32.as_ref())
            .and_then(|m| {
                let share = m.walk().shown()?.0.share;
                m.walk_cells(share).into_iter().find(|c| c.1)
            });
        if let Some((cell, _)) = cell {
            scroll_to(t, Action::Codex32(C32::Cell(cell)));
        }
    };
    to(t, &|s| matches!(s, Step::Sum { .. }));
    marked(t);
    t.shot("roller-codex32-step-sum")?;
    to(t, &|s| matches!(s, Step::Table { .. }));
    marked(t);
    t.shot("roller-codex32-step-table")?;
    // Narrow, More opens the whole card over the page.
    if t.size().0 < 600 {
        c(t, C32::Pane);
        t.shot("roller-codex32-card-open")?;
        c(t, C32::Pane);
    }
    // The view on its way to the next character's cells: a frame 100 ms
    // into the move, nothing settled.
    to(t, &|s| matches!(s, Step::Sum { .. }));
    t.shot_moving("roller-codex32-following", 100)
}
