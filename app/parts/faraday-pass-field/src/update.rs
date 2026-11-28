//! The part's messages and the one place a key changes the field.

use faraday_kernel::{Ctx, Effects, Message, Never, Public};
use faraday_port::event::Key;
use faraday_text::secret::SecretText;

use crate::kind::Refused;
use crate::state::{Last, State, Which};

/// What a press on this part sends; its owner's `Press` holds it.
#[derive(Public, PartialEq)]
pub enum Press {
    /// The box of a field: typing goes to it.
    Box(Which),
    /// The eye.
    Eye,
}

/// Where a key came from: a hardware key's character makes the step wait for the eye to be
/// opened once, the on-screen keyboard's does not.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum By {
    Hardware,
    Screen,
}

#[derive(Message)]
pub enum Msg {
    /// A press on the part.
    Press(Press),
    /// One key; the character is a secret, so the variant is withheld from the log.
    #[withheld]
    Key(Key, By),
    /// Sets the passphrase and its repeat: Practice's `faraday` chip, or a passphrase an owner
    /// carries. Withheld: what it holds is a passphrase, never in the log or a replay.
    #[withheld]
    Fill(SecretText),
    /// Sets the passphrase only; withheld.
    #[withheld]
    FillFirst(SecretText),
    /// Sets the repeat only; withheld.
    #[withheld]
    FillAgain(SecretText),
    /// Wipes both texts, puts typing on the first and closes the eye: the screen is left, a stick
    /// arrived, the entry is given up or done.
    Clear,
    /// The set of passphrase fields drawn changed: the eye closes and nothing waits for it.
    Settled,
}

impl From<Press> for Msg {
    fn from(press: Press) -> Msg {
        Msg::Press(press)
    }
}

/// A stick is in the way of keys and secrets: one is listed, or the kernel shows storage attached
/// that the disk process has not listed.
fn stick(cx: &Ctx<'_>) -> bool {
    let sticks = &cx.view.facts.sticks;
    sticks.attached || !sticks.listed.is_empty()
}

pub fn update(state: &mut State, msg: Msg, cx: &Ctx<'_>) -> Effects<Msg, Never> {
    let stick = stick(cx);
    match msg {
        // A field that takes no typing has no press (it is drawn off).
        Msg::Press(_) if stick => {}
        Msg::Press(Press::Box(which)) => {
            state.on_again = which == Which::Again && state.again.is_some();
        }
        Msg::Press(Press::Eye) => state.eye.toggle(),
        Msg::Key(key, by) => {
            let before = state.bytes();
            state.last = key_in(state, key, stick);
            if by == By::Hardware
                && state.waits_for_eye
                && matches!(key, Key::Char(_))
                && state.last == Last::Typed
                && state.bytes() > before
            {
                state.eye.typed_by_hardware();
            }
            if state.waits_for_eye && state.bytes() == 0 {
                state.eye.reset();
            }
        }
        Msg::Fill(text) => {
            state.text.set(text.as_str());
            if let Some(again) = state.again.as_mut() {
                again.set(text.as_str());
            }
        }
        Msg::FillFirst(text) => state.text.set(text.as_str()),
        Msg::FillAgain(text) => {
            if let Some(again) = state.again.as_mut() {
                again.set(text.as_str());
            }
        }
        Msg::Clear => state.clear(),
        Msg::Settled => {
            state.eye.close();
            state.eye.reset();
        }
    }
    Effects::none()
}

/// Applies one key. The stick rule comes first: with a stick in the way no character, Backspace,
/// Delete, Enter, Tab or Shift-Tab goes in. Then the kind's characters and most, Backspace, and
/// Tab between a pair. Enter and Escape are for the screen.
fn key_in(state: &mut State, key: Key, stick: bool) -> Last {
    if stick
        && matches!(
            key,
            Key::Char(_) | Key::Backspace | Key::Delete | Key::Enter | Key::Tab | Key::BackTab
        )
    {
        return Last::Refused(Refused::Stick);
    }
    match key {
        // A control character is no key of a field.
        Key::Char(c) if c.is_control() => Last::Ignored,
        Key::Char(c) if !state.kind.takes(c) => Last::Refused(Refused::Ascii),
        Key::Char(c) => {
            let most = state.kind.most();
            let at = target(state);
            if most.is_some_and(|m| at.len() + c.len_utf8() > m) {
                return Last::Refused(Refused::TooLong);
            }
            at.push(c);
            Last::Typed
        }
        Key::Backspace => {
            target(state).pop();
            Last::Typed
        }
        Key::Tab | Key::BackTab if state.again.is_some() => {
            state.on_again = !state.on_again;
            Last::Typed
        }
        Key::Enter => Last::Enter,
        Key::Escape => Last::Escape,
        _ => Last::Ignored,
    }
}

/// The text typing goes to.
fn target(state: &mut State) -> &mut SecretText {
    match (state.on_again, state.again.as_mut()) {
        (true, Some(again)) => again,
        _ => &mut state.text,
    }
}
