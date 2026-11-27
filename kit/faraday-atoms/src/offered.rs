//! A control's press, or why it takes none (04 §4, V11).

/// What a control offers: drawn from the one gate (03 A10) by the presentation kernel's `offer`.
#[derive(Clone, Debug, PartialEq)]
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

impl<A: crate::Pressed> crate::ui::Ui<'_, A> {
    /// Draws what `f` draws for a control by what is offered: `f` gets the press the control
    /// takes (none where it is not offered) and whether it is drawn dimmed. A refused control
    /// drawn `Drawn::Dimmed` still takes a press, without a layer, a ring or a Tab place.
    pub fn offered<T>(
        &mut self,
        offered: &Offered<A>,
        f: impl FnOnce(&mut Self, Option<A>, bool) -> T,
    ) -> T {
        match offered {
            Offered::Yes(a) | Offered::Refused(a, Drawn::AsUsual) => {
                f(self, Some(a.clone()), false)
            }
            Offered::Refused(a, Drawn::Dimmed) => {
                self.answering = true;
                let drawn = f(self, Some(a.clone()), true);
                self.answering = false;
                drawn
            }
            Offered::No => f(self, None, true),
        }
    }
}
