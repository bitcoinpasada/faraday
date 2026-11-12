//! Create a vault's use of the dice entry (`dice_entry`): what its
//! keys, ticks and presses do to the passphrases being made. The entry
//! knows nothing of Create a vault; this passes it the focus, the time
//! and the randomness, and wraps its presses as vault actions.

use super::{Focus, VaultAction as V};
use crate::dice_entry::screen::Ask;
use crate::dice_entry::{DiceEntry, Env, Part, Press, Pressed};
use crate::drawing::View;
use crate::ui::Ui;
use crate::{Action, Faraday};
use faraday_shell_api::Key as K;

/// The press a vault action is, if it is one.
fn press_of(a: V) -> Option<Press> {
    Some(match a {
        V::DiceKey(f) => Press::Face(f),
        V::DiceList(k) => Press::List(k),
        V::DiceUse => Press::Use,
        V::DiceClose => Press::Close,
        V::Roller(r) => Press::Roller(r),
        _ => return None,
    })
}

/// A press of the entry, as the action that makes it.
pub(crate) fn wrap(p: Press) -> Action {
    Action::Vault(match p {
        Press::Face(f) => V::DiceKey(f),
        Press::List(k) => V::DiceList(k),
        Press::Use => V::DiceUse,
        Press::Close => V::DiceClose,
        Press::Roller(r) => V::Roller(r),
    })
}

fn focus_of(p: Part) -> Focus {
    match p {
        Part::Dice => Focus::Dice,
        Part::Rolls => Focus::Rolls,
    }
}

/// Runs `f` on the entry with the app's time and randomness, and takes
/// what it says: the focus, and the words Use made.
fn with(app: &mut Faraday, now_ms: u64, f: impl FnOnce(&mut DiceEntry, &mut Env) -> Pressed) {
    let reduce = app.reduce_motion();
    let mut entry = std::mem::take(&mut app.vaults.state.dice);
    let out = f(
        &mut entry,
        &mut Env {
            now_ms,
            reduce,
            draw: &mut || app.fresh(b"faraday dice roller"),
        },
    );
    app.vaults.state.dice = entry;
    apply(app, out);
}

fn apply(app: &mut Faraday, out: Pressed) {
    if let Some(f) = out.focus {
        app.vaults.state.focus = f.map(focus_of);
    }
    if let Some(u) = out.used {
        use_words(app, u);
    }
}

/// A press on the dice entry or the roller.
pub(crate) fn act(app: &mut Faraday, a: V) {
    match a {
        V::Dice(_) if !app.may_load_keys() => {}
        V::Dice(i) => {
            let out = app.vaults.state.dice.start(i);
            apply(app, out);
        }
        V::Roller(_) if !app.roller_offered() => {}
        _ => {
            if let Some(p) = press_of(a) {
                with(app, app.now_ms(), |d, env| d.press(p, env));
            }
        }
    }
}

/// Fills the passphrase with the words the rolls made.
fn use_words(app: &mut Faraday, u: crate::dice_entry::Used) {
    // The demo puts its public passphrase in place of the rolled words,
    // which are dropped (`docs/DEMO.md` §5.2).
    let public = !app.makes_secrets();
    if let Some(c) = app.vaults.state.create.as_mut()
        && let Some((a, b)) = c.phrases.get_mut(u.tag)
        && !u.phrase.is_empty()
    {
        let (phrase, bits) = if public {
            (
                zeroize::Zeroizing::new(crate::demo::PASSPHRASE.to_string()),
                0.0,
            )
        } else {
            (u.phrase, u.bits)
        };
        a.set(&phrase);
        b.set(&phrase);
        if let Some(d) = c.from_dice.get_mut(u.tag) {
            *d = Some((phrase, bits, u.sim));
        }
    }
}

/// The roller's tick: the values it made land on the rolls. Returns
/// whether the screen needs a frame.
pub(crate) fn tick(app: &mut Faraday, now_ms: u64) -> bool {
    let mut redraw = false;
    with(app, now_ms, |d, env| {
        redraw = d.tick(env);
        Pressed::default()
    });
    redraw
}

/// Typing while the rolls or the roller's number has the keys. Returns
/// whether the key was taken.
pub(crate) fn key(app: &mut Faraday, focus: Focus, key: K) -> bool {
    let part = match focus {
        Focus::Rolls => Part::Rolls,
        _ => Part::Dice,
    };
    with(app, app.now_ms(), |d, env| d.key(part, key, env));
    true
}

/// What the dice entry's drawing is given on Create a vault.
pub(crate) fn ask<'a>(app: &'a View<'a>) -> Ask<'a> {
    let (_, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    Ask {
        entry: &app.vaults.dice,
        focus: match app.vaults.focus {
            Some(Focus::Dice) => Some(Part::Dice),
            Some(Focus::Rolls) => Some(Part::Rolls),
            _ => None,
        },
        wrap,
        on_rolls: |i| Action::Vault(V::Dice(i)),
        cost: (mem, passes),
    }
}

/// The height of the input panel at the window's foot while dice are
/// rolled, where it stands (`dice_entry::screen::foot`).
pub(crate) fn foot(app: &View, ui: &Ui) -> Option<f32> {
    let open = app
        .vaults
        .create
        .as_ref()
        .is_some_and(|c| c.open == Some(super::vstep::PHRASES));
    crate::dice_entry::screen::foot(app, ui, &app.vaults.dice, open)
}
