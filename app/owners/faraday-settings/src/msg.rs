use faraday_kernel::{AnyChange, ChangeId, Message};
use faraday_kernel::settings::Choice;
use faraday_port::storage::StickId;
use faraday_present::Pressed;

use crate::clock::Focus;
use crate::kept::ModeAsked;

#[derive(Message)]
#[expect(
    clippy::large_enum_variant,
    reason = "the enum carries the domain's real types, which differ in size; a value is moved once"
)]
pub enum Msg {
    Press(Press),
    /// The boot stick's settings file, read: its bytes, or `None` when it could not be.
    Read(StickId, Option<Vec<u8>>),
    /// A key typed on Set the time (06 §4.3: the product maps the shell's keys).
    Typed(char),
    /// Backspace on Set the time.
    Erase,
    /// Tab on Set the time: the other field.
    Tab,
    /// Enter on Set the time: Set, when a real date and time are typed.
    Enter,
    /// Escape on Set the time: the list of zones closes, else Back.
    Escape,
    #[withheld]
    Given(ChangeId, AnyChange),
}

impl From<Press> for Msg {
    fn from(press: Press) -> Msg {
        Msg::Press(press)
    }
}

pub use faraday_kernel::Top as Out;

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    /// A mode picked on the sheet; the lock's own question follows (`main`'s `mode_act`).
    Mode(ModeAsked),
    /// A mode picked on Settings' Mode card: the Lock sheet asks at once (`Mode`, then
    /// `ModeLock`).
    ModeAsk(ModeAsked),
    /// The mode change is not made: nothing is asked any more.
    ModeCancel,
    /// Lock and use the mode asked for: a lock that keeps nothing but the mode.
    ModeLock,
    /// A choice of the Settings page: appearance, idle times, scale, rotation, what is remembered.
    Choose(Choice),
    /// Open Set the time from Settings.
    OpenSetTime,
    /// Set the time's Back: Settings again.
    SetTimeBack,
    /// Set: the time typed becomes the offset. Needs a zone and a real date and time.
    Set,
    /// Keep this time: the clock counts as checked, no offset.
    Keep,
    /// A field of Set the time takes the typing.
    Focus(Focus),
    /// Show or hide the list of zones.
    ZoneList(bool),
    /// Move the zone by this many minutes (a quarter hour either way).
    ZoneStep(i16),
    /// Choose the zone this many minutes east of UTC from the list.
    Zone(i16),
    /// Run the self-test again.
    SelfTest,
    /// The search field is pressed: the typing goes to it.
    Find,
    /// The search field's x.
    FindClear,
    /// Lock, which asks first where something would be lost: the power feature's (the product
    /// routes it).
    Lock,
    /// Power off or Quit: the power feature's Power sheet (the product routes it).
    PowerAsk,
}
