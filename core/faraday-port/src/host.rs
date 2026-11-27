//! The desktop computer's asks and answers (02 §4.3).

use faraday_text::Text;

use crate::ask::{Failure, request};
use crate::bytes::Bytes;

pub enum HostAsk {
    /// A file in Downloads or one dropped on the window, for Transfer.
    Read { path: Text },
    /// A received file into Downloads: never over a file; `name (2).ext`
    /// and on, up to 999.
    Save { name: Text, bytes: Bytes },
    Open(Openable),
}

pub enum HostAnswer {
    Read(Result<Bytes, Failure>),
    /// The path.
    Saved(Result<Text, Failure>),
    Opened(Result<(), Failure>),
}

/// What the computer may open: no address the app composes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Openable {
    Downloads,
    /// The one fixed address `main` opens (the Settings About note).
    Institute,
    /// A PDF the Starter saved.
    File(Text),
}

/// `HostAsk::Read`'s own type.
pub struct Read {
    pub path: Text,
}
request!(Read => Result<Bytes, Failure>);

/// `HostAsk::Save`'s own type.
pub struct Save {
    pub name: Text,
    pub bytes: Bytes,
}
request!(Save => Result<Text, Failure>);

/// `HostAsk::Open`'s own type.
pub struct Open(pub Openable);
request!(Open => Result<(), Failure>);
