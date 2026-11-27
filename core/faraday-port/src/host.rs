//! The desktop computer's asks and answers (02 §4.3).

use faraday_text::Text;

use crate::ask::{Answer, Ask, Failure, request};
use crate::bytes::Bytes;

pub enum HostAsk {
    /// A file in Downloads or one dropped on the window, for Transfer.
    Read {
        path: Text,
    },
    /// A received file into Downloads: never over a file; `name (2).ext`
    /// and on, up to 999.
    Save {
        name: Text,
        bytes: Bytes,
    },
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
request!(Read => Result<Bytes, Failure>;
    into a => Ask::Host(HostAsk::Read { path: a.path });
    take r @ Answer::Host(HostAnswer::Read(r)),
    fault Err(Failure::Adapter));

/// `HostAsk::Save`'s own type.
pub struct Save {
    pub name: Text,
    pub bytes: Bytes,
}
request!(Save => Result<Text, Failure>;
    into a => Ask::Host(HostAsk::Save { name: a.name, bytes: a.bytes });
    take r @ Answer::Host(HostAnswer::Saved(r)),
    fault Err(Failure::Adapter));

/// `HostAsk::Open`'s own type.
pub struct Open(pub Openable);
request!(Open => Result<(), Failure>;
    into a => Ask::Host(HostAsk::Open(a.0));
    take r @ Answer::Host(HostAnswer::Opened(r)),
    fault Err(Failure::Adapter));
