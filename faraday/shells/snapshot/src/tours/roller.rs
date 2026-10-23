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
    t.shot("roller-used")
}
