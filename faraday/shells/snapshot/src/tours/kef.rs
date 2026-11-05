//! The scanned Krux encrypted code tour (`docs/KEF.md` §15): its card,
//! the passphrase sheet and the one failure message.

use crate::tour::{Tour, type_text};
use faraday_core::{Action, Screen};
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
