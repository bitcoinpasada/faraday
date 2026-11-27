//! The helpers every `present` shares (04 §2).

use faraday_kernel::{Feature, JobKind, Line, Refused, Running, Show, View};
use faraday_organisms::{BusyModel, Offered, PassModel, Pressed, Shown};
use faraday_pass_field::Press as PassPress;

/// A control's press or why not, from the one gate (A10, V11).
pub fn offer<P: Pressed, F: Feature>(
    state: &F::State,
    msg: &F::Msg,
    press: P,
    view: &View<'_>,
) -> Offered<P> {
    match F::refuses(state, msg, view) {
        None => Offered::Yes(press),
        Some(refusal) => Offered::Refused(press, refused(&refusal)),
    }
}

/// A worded line as the kit's text.
pub fn line<'s>(l: &'s Line) -> Shown<'s> {
    match l {
        Line::Fixed(text) => Shown::plain(*text),
        Line::Filled(text) => Shown::plain(text),
    }
}

/// The busy line of a feature's job kind, none when no such job runs or it is quiet.
pub fn busy<'s>(_running: &[Running], _kind: JobKind, _show: &Show<'s>) -> Option<BusyModel<'s>> {
    todo!()
}

/// The passphrase part's read view mapped to the kit's model; the text only while the eye is
/// open, through `Shown::secret`.
pub fn pass_model<'s, A: Pressed>(
    _v: faraday_pass_field::Read<'s>,
    _to: fn(PassPress) -> A,
) -> PassModel<'s, A> {
    todo!()
}

/// `Refused::drawn` to the kit's own `Drawn`.
pub fn refused(r: &impl Refused) -> faraday_organisms::Drawn {
    match r.drawn() {
        faraday_kernel::Drawn::Dimmed => faraday_organisms::Drawn::Dimmed,
        faraday_kernel::Drawn::AsUsual => faraday_organisms::Drawn::AsUsual,
    }
}
