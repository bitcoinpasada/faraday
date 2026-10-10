//! What the removable medium is called. The PC's is a stick (a USB stick,
//! or an SD card in a USB reader); the Pi has no USB storage, so every
//! medium it sees is an SD card. The shell says which at start, and every
//! string naming the medium takes its words from here: the noun, its
//! article, its plural and the forms that start a sentence, so that no
//! string builds "a {noun}" by hand.

use osk_ui::widgets::Icon;

/// The medium files move on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Medium {
    /// A USB stick: the PC, the stick image and the desktop app.
    #[default]
    Stick,
    /// An SD card: the Pi.
    SdCard,
}

impl Medium {
    /// The noun mid-sentence: "stick", "SD card".
    pub const fn noun(self) -> &'static str {
        match self {
            Medium::Stick => "stick",
            Medium::SdCard => "SD card",
        }
    }

    /// The plural mid-sentence: "sticks", "SD cards".
    pub const fn nouns(self) -> &'static str {
        match self {
            Medium::Stick => "sticks",
            Medium::SdCard => "SD cards",
        }
    }

    /// With its article mid-sentence: "a stick", "an SD card".
    pub const fn a(self) -> &'static str {
        match self {
            Medium::Stick => "a stick",
            Medium::SdCard => "an SD card",
        }
    }

    /// The noun starting a label or sentence: "Stick", "SD card".
    pub const fn cap(self) -> &'static str {
        match self {
            Medium::Stick => "Stick",
            Medium::SdCard => "SD card",
        }
    }

    /// The plural starting a label or sentence: "Sticks", "SD cards".
    pub const fn caps(self) -> &'static str {
        match self {
            Medium::Stick => "Sticks",
            Medium::SdCard => "SD cards",
        }
    }

    /// With its article starting a sentence: "A stick", "An SD card".
    pub const fn a_cap(self) -> &'static str {
        match self {
            Medium::Stick => "A stick",
            Medium::SdCard => "An SD card",
        }
    }

    /// Its full name mid-sentence, where the kind matters: "USB stick",
    /// "SD card".
    pub const fn full(self) -> &'static str {
        match self {
            Medium::Stick => "USB stick",
            Medium::SdCard => "SD card",
        }
    }

    /// Its full name with its article, starting a sentence: "A USB
    /// stick", "An SD card".
    pub const fn a_full_cap(self) -> &'static str {
        match self {
            Medium::Stick => "A USB stick",
            Medium::SdCard => "An SD card",
        }
    }

    /// The screen that writes the Outbox to it and copies files in:
    /// "Stick visit", "SD card visit".
    pub const fn visit(self) -> &'static str {
        match self {
            Medium::Stick => "Stick visit",
            Medium::SdCard => "SD card visit",
        }
    }

    /// Settings' way into upgrading another Faraday medium from the one
    /// the device started from (`PLAN.md` §5.5), and the screen's name.
    pub const fn upgrade(self) -> &'static str {
        match self {
            Medium::Stick => "Upgrade a Faraday stick",
            Medium::SdCard => "Upgrade a Faraday SD card",
        }
    }

    /// What the medium the device started from is called.
    pub const fn boot(self) -> &'static str {
        match self {
            Medium::Stick => "Boot stick",
            Medium::SdCard => "Boot SD card",
        }
    }

    /// Its glyph.
    pub const fn icon(self) -> Icon {
        match self {
            Medium::Stick => Icon::Drive,
            Medium::SdCard => Icon::SdCard,
        }
    }
}
