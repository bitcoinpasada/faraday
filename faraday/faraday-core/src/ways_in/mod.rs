//! The ways a key reaches a screen: one component for every screen that
//! takes a key (`docs/KEY-FORMS.md` §4).
//!
//! The table gives it the forms; an [`Opener`] says which forms its screen
//! takes and which ways show first; everything the opener takes that is not
//! shown first is under Other forms, worked out here so no screen lists it
//! by hand.

pub(crate) mod screen;

use crate::forms::Form;

/// A way a key reaches a screen. A Form way is any form of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Way {
    /// Scan a code: it switches form itself (§3.3).
    Scan,
    /// One form of the table.
    Form(Form),
}

/// What a screen takes and what it shows first.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Opener {
    /// The forms the screen takes.
    pub forms: &'static [Form],
    /// The ways shown first, in order.
    pub up_front: &'static [Way],
}

/// Add a key: every form; Scan, the word grid and codex32 first.
pub(crate) const ADD_A_KEY: Opener = Opener {
    forms: &Form::ALL,
    up_front: &[Way::Scan, Way::Form(Form::Words), Way::Form(Form::Codex32)],
};

impl Opener {
    /// The forms shown first, in order.
    pub(crate) fn up_front_forms(&self) -> impl Iterator<Item = Form> + '_ {
        self.up_front.iter().filter_map(|w| match w {
            Way::Form(f) => Some(*f),
            Way::Scan => None,
        })
    }

    /// The forms under Other forms: the forms the opener takes less those
    /// shown first.
    pub(crate) fn other_forms(&self) -> Vec<Form> {
        self.forms
            .iter()
            .copied()
            .filter(|f| !self.up_front.contains(&Way::Form(*f)))
            .collect()
    }
}
