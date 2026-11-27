//! A control's press, or why it takes none (04 §4, V11).

/// What a control offers: drawn from the one gate (03 A10) by the presentation kernel's `offer`.
pub enum Offered<R> {
    /// Drawn enabled; a press sends `R`.
    Yes(R),
    /// Refused: drawn as `Drawn` says, and a press still sends `R`, which `route` answers with
    /// the refusal's note.
    Refused(R, Drawn),
    /// Drawn dimmed; it takes no press.
    No,
}

/// The kernel's `Refused::drawn` answer, mapped by the presentation kernel (03 §10).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Drawn {
    Dimmed,
    AsUsual,
}
