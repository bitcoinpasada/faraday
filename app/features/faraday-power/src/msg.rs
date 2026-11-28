use faraday_kernel::{FrameSheet, Message, Public, Top};
use faraday_present::Pressed;

/// The sheets the feature shows.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Which {
    /// Power off, and Restart where the computer offers it.
    Power,
    /// The Lock sheet: what a lock would lose, asked before it.
    LockAsk,
    /// What the next app tells after a lock.
    Locked,
    /// The warning before the idle lock.
    IdleWarn,
}

impl Which {
    /// The sheet's anchor (`ANCHORS`, 06 §4.2).
    #[must_use]
    pub fn anchor(self) -> faraday_port::values::Anchor {
        let at = match self {
            Which::LockAsk => 1,
            Which::Locked => 2,
            Which::Power => 3,
            Which::IdleWarn => 4,
        };
        crate::ANCHORS[at]
    }

    /// The frame's stack slot of the sheet (06 §4.4).
    #[must_use]
    pub fn frame(self) -> FrameSheet {
        match self {
            Which::Power => FrameSheet::Power,
            Which::LockAsk => FrameSheet::LockAsk,
            Which::Locked => FrameSheet::Lock,
            Which::IdleWarn => FrameSheet::IdleWarn,
        }
    }
}

#[derive(Message)]
pub enum Msg {
    Press(Press),
    /// The parent shows a sheet: the Power control's, the idle warning's.
    Show(Which),
    /// A Lock was asked for anywhere: with `asks_first` (what the parent read of `holds`) the
    /// Lock sheet comes first, else the lock begins.
    Lock {
        asks_first: bool,
    },
    /// The settings owner asked for a mode change: the Lock sheet comes first whether or not
    /// anything would be lost, and its Lock seals (`main`'s `mode_act` into `lock_ask`).
    ModeLock,
}

impl From<Press> for Msg {
    fn from(press: Press) -> Msg {
        Msg::Press(press)
    }
}

/// What the feature tells its parent.
#[derive(Message)]
pub enum Out {
    /// A frame sheet.
    Front(Option<FrameSheet>),
    /// A lock, an end, a broadcast.
    Top(Top),
}

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    /// Cancel, Continue, Keep working: the sheet goes away.
    Away,
    /// Lock, sealing the open vaults first.
    Lock,
    /// Lock, losing what cannot be sealed.
    LockWithoutSealing,
    Restart,
    /// Power off; Quit in the desktop app.
    PowerOff,
}
