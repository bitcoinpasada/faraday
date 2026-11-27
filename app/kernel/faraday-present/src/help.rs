//! The helpers every `present` shares (04 §2).

use faraday_kernel::{Feature, JobKind, Line, Refused, Running, Show, View};
use faraday_organisms::{BusyModel, Offered, PassModel, Pressed, Shown};
use faraday_pass_field::Press as PassPress;

/// A control's press or why not, from the one gate (A10, V11).
pub fn offer<P: Pressed, F: Feature>(
    _state: &F::State,
    _msg: &F::Msg,
    _press: P,
    _view: &View<'_>,
) -> Offered<P> {
    todo!()
}

/// A worded line as the kit's text.
pub fn line<'s>(_l: &'s Line) -> Shown<'s> {
    todo!()
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
pub fn refused(_r: &impl Refused) -> faraday_organisms::Drawn {
    todo!()
}
