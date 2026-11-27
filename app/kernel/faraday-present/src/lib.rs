//! The presentation kernel: `Present`, `Page`, `ShowFlow`, the step-flow fill and the helpers
//! every `present` shares. Section 09 §4.2 and section 04 §2 to §5.

pub mod flow;
pub mod help;
pub mod screen;
pub mod stages;

pub use flow::{ChoiceLine, FlowPress, PressOf, ShowFlow, fill};
pub use help::{busy, line, offer, pass_model, refused};
pub use screen::{Drawn, ScreenModel, draw};
pub use stages::{Page, Present};
