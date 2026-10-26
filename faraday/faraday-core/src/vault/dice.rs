//! Generate with dice on Create a vault: the rolls typed, and beside
//! them, on Faraday Demo, the roller (`docs/CODEX32.md` §5.6). A
//! rolled value lands on the rolls as a key pressed would, and is marked
//! as this computer's, so that the passphrase's strength line says
//! "simulated dice".

use super::{Focus, STRONG_BITS, TextBox, VaultAction as V, dice_words};
use crate::Faraday;
use crate::fresh::roller::{Act, PerClick};
use faraday_shell_api::Key as K;
use zeroize::Zeroizing;

/// The most words the roller adds, one click or many.
const MAX_WORDS: usize = 20;

/// How many more rolls the entry takes now: up to [`MAX_WORDS`] words'
/// worth, and for To the end only as far as a strong passphrase (a
/// lookup).
pub(crate) fn room(app: &Faraday) -> usize {
    let Some((_, rolls, list)) = app.vaults.dice.as_ref() else {
        return 0;
    };
    let per = list.dice_per_word();
    let end = if app.vaults.roller.per() == PerClick::ToEnd {
        (STRONG_BITS as f32 / list.bits_per_word()).ceil() as usize
    } else {
        MAX_WORDS
    };
    (end * per).saturating_sub(rolls.text.len())
}

/// A press on the dice entry or the roller.
pub(super) fn act(app: &mut Faraday, a: V) {
    match a {
        V::Dice(_) if !app.may_load_keys() => {}
        V::Dice(i) => {
            // Pressed again, the rolls box keeps what is rolled.
            if app.vaults.dice.as_ref().is_none_or(|d| d.0 != i) {
                app.vaults.state.dice =
                    Some((i, TextBox::default(), faraday_bip::diceware::List::Large));
                app.vaults.state.roller.reset();
            }
            app.vaults.state.focus = Some(Focus::Dice);
        }
        V::DiceList(k) => {
            if let Some((_, rolls, list)) = app.vaults.state.dice.as_mut()
                && let Some(&l) = faraday_bip::diceware::List::ALL.get(usize::from(k))
                && l != *list
            {
                *list = l;
                rolls.clear();
                app.vaults.state.roller.reset();
            }
        }
        V::DiceUse => use_words(app),
        V::DiceClose => {
            app.vaults.state.dice = None;
            app.vaults.state.focus = None;
            app.vaults.state.roller.reset();
        }
        V::Roller(r) if app.roller_offered() => roll(app, r),
        _ => {}
    }
}

/// Fills the passphrase with the words the rolls made.
fn use_words(app: &mut Faraday) {
    let Some((i, rolls, list)) = app.vaults.state.dice.take() else {
        return;
    };
    let words = dice_words(&rolls.text, list);
    let sim = app.vaults.roller.sim(words.len() * list.dice_per_word());
    // The demo puts its public passphrase in place of the rolled words,
    // which are dropped (`docs/DEMO.md` §5.2).
    let public = !app.makes_secrets();
    if let Some(c) = app.vaults.state.create.as_mut()
        && let Some((a, b)) = c.phrases.get_mut(i)
        && !words.is_empty()
    {
        let (phrase, bits) = if public {
            (Zeroizing::new(crate::demo::PASSPHRASE.to_string()), 0.0)
        } else {
            (
                Zeroizing::new(words.join("")),
                words.len() as f32 * list.bits_per_word(),
            )
        };
        a.set(&phrase);
        b.set(&phrase);
        if let Some(d) = c.from_dice.get_mut(i) {
            *d = Some((phrase, bits, sim));
        }
    }
    app.vaults.state.roller.reset();
    app.vaults.state.focus = None;
}

/// A press on the roller: its choice, or Roll. Under Reduce motion the
/// values land at once.
fn roll(app: &mut Faraday, r: Act) {
    let (room, now) = (room(app), app.now_ms());
    app.vaults.state.roller.act(r, room, now);
    app.vaults.state.focus = Some(if r == Act::Count {
        Focus::Rolls
    } else {
        Focus::Dice
    });
    if r == Act::Roll && app.reduce_motion() {
        tick(app, now);
    }
}

/// The roller's tick: the values it made land on the rolls. Returns
/// whether the screen needs a frame.
pub(super) fn tick(app: &mut Faraday, now_ms: u64) -> bool {
    if !app.vaults.roller.active() {
        return false;
    }
    let (reduce, room) = (app.reduce_motion(), room(app));
    let Some(len) = app.vaults.dice.as_ref().map(|d| d.1.text.len()) else {
        app.vaults.state.roller.reset();
        return false;
    };
    let mut roller = std::mem::take(&mut app.vaults.state.roller);
    let (values, redraw) = roller.tick(now_ms, reduce, len, room, &mut || {
        app.fresh(b"faraday dice roller")
    });
    app.vaults.state.roller = roller;
    if let Some((_, rolls, _)) = app.vaults.state.dice.as_mut() {
        for v in values {
            rolls.text.push(char::from(b'0' + v));
        }
    }
    redraw
}

/// Typing while the rolls or the roller's number has the keys. Returns
/// whether the key was taken.
pub(super) fn key(app: &mut Faraday, focus: Focus, key: K) -> bool {
    if focus == Focus::Rolls {
        match key {
            K::Char(c) => app.vaults.state.roller.digit(c),
            K::Backspace => app.vaults.state.roller.backspace(),
            K::Enter => app.vaults.state.focus = Some(Focus::Dice),
            _ => {}
        }
        return true;
    }
    if let Some((_, rolls, _)) = app.vaults.state.dice.as_mut() {
        match key {
            K::Char(c) if ('1'..='6').contains(&c) => rolls.text.push(c),
            K::Backspace => {
                rolls.text.pop();
                let len = rolls.text.len();
                app.vaults.state.roller.trim(len);
            }
            K::Enter => act(app, V::DiceUse),
            _ => {}
        }
    }
    true
}
