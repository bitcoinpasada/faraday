//! Refusals and the wording (03 §10).

use faraday_port::Failure;
use faraday_text::Text;

use crate::cx::Show;
use crate::message::Public;

/// A typed value the person is told about. Implemented in the feature's `present` module, by an
/// exhaustive match. Object-safe: a `Note` boxes one. Worded only where a `Show` is, so never in
/// `update`.
pub trait Worded: Send + 'static {
    fn words(&self, show: &Show<'_>) -> Line;
}

/// `Filled` is `faraday_strings::fill` of a template.
pub enum Line {
    Fixed(&'static str),
    Filled(Text),
}

/// A refusal: worded, and how a control it refuses is drawn.
pub trait Refused: Worded + Public {
    fn drawn(&self) -> Drawn {
        Drawn::Dimmed
    }
}

/// How a refused control is drawn. The presentation kernel maps it to the kit's own, since the
/// kernel may not name the kit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Drawn {
    Dimmed,
    AsUsual,
}

/// The kernel words `port::Failure` once, so a failed read or write reads alike everywhere.
impl Worded for Failure {
    fn words(&self, _show: &Show<'_>) -> Line {
        todo!()
    }
}
