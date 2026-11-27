//! The application kernel (03 §1): features and their messages, effects, `Ctx` and `View`, the
//! shared owners' traits, step flows, locks, navigation, handles and the wording of refusals.
//! It holds no feature's types (A1) and names no root and no kit crate.

#![expect(dead_code, reason = "the fields are read once the bodies are written")]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

// The derives write `::faraday_kernel::..` paths; inside this crate they name the crate itself.
extern crate self as faraday_kernel;

pub mod cx;
pub mod effects;
pub mod feature;
pub mod flow;
pub mod handle;
pub mod lock;
pub mod message;
pub mod nav;
pub mod shared;
pub mod words;

/// What the derives name; `lint-secrets` refuses this path anywhere but in derive output (A14).
#[doc(hidden)]
pub mod __private {
    pub trait Sealed {}
}

pub use cx::{
    Clock, Ctx, Derived, Downloads, Edits, Facts, FactsChanged, Memo, Mode, NoteUp, Running,
    SelfTest, Show, Slot, Started, Sticks, SystemFacts, View,
};
pub use effects::{
    AskFor, ChangeFor, ChangeReply, Effects, Fault, Fx, Job, JobKind, JobParts, Note, Reply, Route,
    ViewOrder,
};
pub use feature::{End, Feature, Lock, Notice, Top, route};
pub use flow::{
    Back, CancelOffer, Ending, Flow, FlowFx, FlowMsg, FlowRefusal, StepFlow, Steps, Width,
};
pub use handle::{FileId, Gate, KeyId, SampleAsk, ScanFor, Ticket, Tool, Waiting, WalletId};
pub use lock::{AtLock, Carried, CarryName, CarryValue, FileBox, Holds, Loss};
pub use message::{Message, Never, Public, Withheld};
pub use nav::{Frame, FrameSheet, Holder, Nav};
pub use shared::{AnyChange, Change, ChangeId, Done, Owner, session, settings, vaults};
pub use words::{Drawn, Line, Refused, Worded};
