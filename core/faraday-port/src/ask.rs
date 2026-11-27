//! Asks, answers and how an answer finds its request (02 §2, §4, §7.2).

use alloc::vec::Vec;
use core::fmt;

use faraday_text::{Text, secret::SecretText};

use crate::bytes::Entropy as EntropyAnswer;
use crate::camera;
use crate::host::{HostAnswer, HostAsk};
use crate::storage::{Carry, StorageAnswer, StorageAsk};
use crate::system::{SystemAnswer, SystemAsk};

/// Minted by the runtime only. `generation` counts the builds of the
/// lock scope in this process, from 1 (a lock in Faraday Demo or the
/// Starter's Faraday Demo builds the scope again in process); the part
/// outside the scope (the Starter's own composite and its settings owner)
/// asks with generation 0. `n` counts asks within one generation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AskId {
    pub generation: u32,
    pub n: u32,
}

/// Every ask's own type implements this; the answer type is the ask's.
pub trait Request: Into<Ask> {
    type Answer;
    /// Takes this ask's answer out of the port's enum; any other variant
    /// is the adapter's fault.
    #[expect(
        clippy::result_large_err,
        reason = "the port's enums carry the domain's real types, which differ in size; a value is moved once through the queue"
    )]
    fn take(answer: Answer) -> Result<Self::Answer, Answer>;
    /// This ask's answer when the adapter gave the wrong one (P9).
    fn fault() -> Self::Answer;
}

/// Gives a request type its `Into<Ask>` and `Request`. `into` builds the port's `Ask`, `take` names
/// the one `Answer` variant that answers it, and `fault` is the answer for any other (P9). The
/// one-argument form is for an ask no product's path carries yet: its bodies are `todo!()`.
macro_rules! request {
    ($ask:ty => $answer:ty) => {
        impl From<$ask> for $crate::ask::Ask {
            fn from(_ask: $ask) -> $crate::ask::Ask {
                todo!()
            }
        }

        impl $crate::ask::Request for $ask {
            type Answer = $answer;

            fn take(_answer: $crate::ask::Answer) -> Result<Self::Answer, $crate::ask::Answer> {
                todo!()
            }

            fn fault() -> Self::Answer {
                todo!()
            }
        }
    };
    (
        $ask:ty => $answer:ty;
        into $a:ident => $into:expr;
        take $r:ident @ $pat:pat,
        fault $fault:expr
    ) => {
        impl From<$ask> for $crate::ask::Ask {
            fn from($a: $ask) -> $crate::ask::Ask {
                $into
            }
        }

        impl $crate::ask::Request for $ask {
            type Answer = $answer;

            fn take(answer: $crate::ask::Answer) -> Result<Self::Answer, $crate::ask::Answer> {
                match answer {
                    $pat => Ok($r),
                    other => Err(other),
                }
            }

            fn fault() -> Self::Answer {
                $fault
            }
        }
    };
}
pub(crate) use request;

pub enum Ask {
    Storage(StorageAsk),
    Keep(Carry),
    Forget,
    Host(HostAsk),
    Camera(camera::On),
    Clipboard,
    Print(PrintJob),
    Entropy,
    Memory,
    Settings(Text),
    System(SystemAsk),
}

pub enum Answer {
    Storage(StorageAnswer),
    Kept(Result<(), (Failure, Carry)>),
    Forgotten(Result<(), Failure>),
    Host(HostAnswer),
    Camera(Result<camera::Started, camera::Failure>),
    Clipboard(Result<SecretText, Failure>),
    /// Where the PDF went.
    Printed(Result<Text, Failure>),
    Entropy(Result<EntropyAnswer, Failure>),
    /// MiB free now.
    Memory(Option<u32>),
    Settings(Result<(), Failure>),
    System(SystemAnswer),
}

/// The variant names of an `Ask`, which the runtime logs (P15).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AskKind {
    Storage,
    Keep,
    Forget,
    Host,
    Camera,
    Clipboard,
    Print,
    Entropy,
    Memory,
    Settings,
    System,
}

impl Ask {
    pub fn kind(&self) -> AskKind {
        match self {
            Ask::Storage(_) => AskKind::Storage,
            Ask::Keep(_) => AskKind::Keep,
            Ask::Forget => AskKind::Forget,
            Ask::Host(_) => AskKind::Host,
            Ask::Camera(_) => AskKind::Camera,
            Ask::Clipboard => AskKind::Clipboard,
            Ask::Print(_) => AskKind::Print,
            Ask::Entropy => AskKind::Entropy,
            Ask::Memory => AskKind::Memory,
            Ask::Settings(_) => AskKind::Settings,
            Ask::System(_) => AskKind::System,
        }
    }
}

/// Names only (P16): an ask carries files, text and bytes that may be secret.
impl fmt::Debug for Ask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ask::{:?}", self.kind())
    }
}

impl Answer {
    /// The variant's name, which is all `Debug` prints (P16).
    pub fn name(&self) -> &'static str {
        match self {
            Answer::Storage(_) => "Storage",
            Answer::Kept(_) => "Kept",
            Answer::Forgotten(_) => "Forgotten",
            Answer::Host(_) => "Host",
            Answer::Camera(_) => "Camera",
            Answer::Clipboard(_) => "Clipboard",
            Answer::Printed(_) => "Printed",
            Answer::Entropy(_) => "Entropy",
            Answer::Memory(_) => "Memory",
            Answer::Settings(_) => "Settings",
            Answer::System(_) => "System",
        }
    }
}

impl fmt::Debug for Answer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Answer::{}", self.name())
    }
}

/// `Ask::Keep`'s own type.
pub struct Keep(pub Carry);
request!(Keep => Result<(), (Failure, Carry)>;
    into a => Ask::Keep(a.0);
    take r @ Answer::Kept(r),
    fault Err((Failure::Adapter, Carry { inbox: Vec::new(), outbox: Vec::new(), kept: Vec::new() })));

/// `Ask::Forget`'s own type.
pub struct Forget;
request!(Forget => Result<(), Failure>;
    into _a => Ask::Forget;
    take r @ Answer::Forgotten(r),
    fault Err(Failure::Adapter));

/// `Ask::Clipboard`'s own type.
pub struct Clipboard;
request!(Clipboard => Result<SecretText, Failure>;
    into _a => Ask::Clipboard;
    take r @ Answer::Clipboard(r),
    fault Err(Failure::Adapter));

/// `Ask::Entropy`'s own type.
pub struct Entropy;
request!(Entropy => Result<EntropyAnswer, Failure>;
    into _a => Ask::Entropy;
    take r @ Answer::Entropy(r),
    fault Err(Failure::Adapter));

/// `Ask::Memory`'s own type.
pub struct Memory;
request!(Memory => Option<u32>;
    into _a => Ask::Memory;
    take r @ Answer::Memory(r),
    fault None);

/// `Ask::Settings`'s own type: the app's settings text, kept unread.
pub struct Settings(pub Text);
request!(Settings => Result<(), Failure>;
    into a => Ask::Settings(a.0);
    take r @ Answer::Settings(r),
    fault Err(Failure::Adapter));

/// What to print, and `Ask::Print`'s own type (02 §4.6).
pub struct PrintJob {
    pub name: Text,
    pub pdf: crate::bytes::Bytes,
    pub holds_secret: bool,
}
request!(PrintJob => Result<Text, Failure>;
    into a => Ask::Print(a);
    take r @ Answer::Printed(r),
    fault Err(Failure::Adapter));

/// Why an ask failed (02 §7.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// This product does not carry it.
    Unsupported,
    /// Carried, but nothing here now (no camera, no randomness).
    Unavailable,
    /// P20.
    Busy,
    /// The disk process or a helper did not answer in time.
    Silent,
    /// The stick, the file or the device went.
    Gone,
    TooLarge,
    NoRoom,
    ReadOnly,
    NotFat,
    /// An answer from the disk process the adapter could not read.
    Malformed,
    /// The clipboard held nothing usable.
    Empty,
    /// The person declined an elevation or a prompt.
    Declined,
    /// P9: the wrong answer for the ask.
    Adapter,
    /// P11: a failed write's temporary file, by name, that could not be removed.
    LeftBehind(Text),
    /// The operating system's code, for diagnostics.
    Os(OsError),
}

/// The operating system's own error code, as `std::io::Error::raw_os_error` gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OsError(pub i32);
