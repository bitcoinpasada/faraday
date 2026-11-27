//! A control's press, or why not (04 §4, §11).

/// A control's press or why not.
pub enum Offered<R> {
    Yes(R),
    Refused(Drawn),
    Busy,
}

/// The kernel's `Refused::drawn` answer, mapped by the presentation kernel (03 §10).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Drawn {
    Dimmed,
    AsUsual,
}
