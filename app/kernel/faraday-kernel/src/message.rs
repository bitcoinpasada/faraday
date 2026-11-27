//! `Message`, `Public` and `Withheld`, and the derives (03 §3).

use core::fmt::Debug;

use faraday_bip::keys::{Fingerprint, Network};
use faraday_port::AskId;
use faraday_port::event::EventKind;
use faraday_port::storage::{Sha256, StickId};
use faraday_port::values::Anchor;
use faraday_text::Text;

use crate::__private::Sealed;

pub use faraday_kernel_derive::{Message, Public};

/// Every message type. Derived.
pub trait Message: Send + 'static {
    /// The variant's path, such as "create.Forward"; always logged.
    fn kind(&self) -> &'static str;
    /// The message whole, when its variant is Public; `None` when withheld.
    fn public(&self) -> Option<&dyn Debug>;
}

/// Holds no secret: may be logged, replayed, cloned and kept in a reply's context. Sealed (§3.2).
pub trait Public: Clone + Debug + Send + 'static + Sealed {}

/// The mark of a variant whose payload is never logged: its kind alone is.
// Spec: 03 §1 lists `Withheld` among the message module's items without a shape.
pub struct Withheld;

macro_rules! public {
    ($($t:ty),* $(,)?) => {
        $(
            impl Sealed for $t {}
            impl Public for $t {}
        )*
    };
}

public!(
    bool,
    char,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    Text,
    Fingerprint,
    Network,
    Sha256,
    AskId,
    StickId,
    EventKind,
    Anchor,
);

impl<T: Public> Sealed for Option<T> {}
impl<T: Public> Public for Option<T> {}
impl<T: Public> Sealed for Vec<T> {}
impl<T: Public> Public for Vec<T> {}
impl<T: Public, const N: usize> Sealed for [T; N] {}
impl<T: Public, const N: usize> Public for [T; N] {}

macro_rules! public_tuples {
    ($(($($t:ident),+))*) => {
        $(
            impl<$($t: Public),+> Sealed for ($($t,)+) {}
            impl<$($t: Public),+> Public for ($($t,)+) {}
        )*
    };
}

public_tuples!((A)(A, B)(A, B, C)(A, B, C, D));

/// The out of a part: it tells its owner nothing (03 §1; 09 §4.3).
// Spec: 09 §4.3 writes `Effects<Msg, Never>` without defining `Never`.
pub enum Never {}

impl Message for Never {
    fn kind(&self) -> &'static str {
        todo!()
    }

    fn public(&self) -> Option<&dyn Debug> {
        todo!()
    }
}
