//! The helpers every `present` shares (04 §2).

use faraday_kernel::{Feature, JobKind, Line, Refused, Running, Show, View};
use faraday_organisms::{BusyModel, Offered, PassModel, PassView, Pressed, Shown};
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
/// open, through `Shown::secret`. A field that takes no typing (a stick in the way) offers
/// neither its box nor its eye.
pub fn pass_model<'s, A: Pressed>(
    v: faraday_pass_field::Read<'s>,
    to: fn(PassPress) -> A,
) -> PassModel<'s, A> {
    let typing = v.typing();
    let view = match v.revealed() {
        Some(text) => PassView::revealed(text),
        None => PassView::dots(v.count()),
    };
    let offer = |press: PassPress| {
        if typing {
            Offered::Yes(to(press))
        } else {
            Offered::No
        }
    };
    let tip = if v.shown() {
        faraday_strings::EN.vault_hide_passphrases
    } else {
        faraday_strings::EN.vault_show_passphrases
    };
    PassModel {
        view,
        focused: v.focused(),
        marked: v.marked(),
        hint: None,
        none: None,
        eye_tip: Shown::plain(tip),
        press: offer(PassPress::Box(v.which())),
        eye: offer(PassPress::Eye),
    }
}

/// `Refused::drawn` to the kit's own `Drawn`.
pub fn refused(r: &impl Refused) -> faraday_organisms::Drawn {
    match r.drawn() {
        faraday_kernel::Drawn::Dimmed => faraday_organisms::Drawn::Dimmed,
        faraday_kernel::Drawn::AsUsual => faraday_organisms::Drawn::AsUsual,
    }
}
