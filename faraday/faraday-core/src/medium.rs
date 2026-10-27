//! What the removable medium is called. The PC's is a stick (a USB stick,
//! or an SD card in a USB reader); the Pi has no USB storage, so every
//! medium it sees is an SD card. The shell says which at start, and every
//! string naming the medium takes its words from here: the noun, its
//! article, its plural and the forms that start a sentence, so that no
//! string builds "a {noun}" by hand.

use crate::strings::EN;
use crate::ui::Glyph;
use faraday_draw::widgets::Icon;

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
            Medium::Stick => EN.medium_stick_inline,
            Medium::SdCard => EN.medium_sd_card,
        }
    }

    /// The plural mid-sentence: "sticks", "SD cards".
    pub const fn nouns(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_sticks_inline,
            Medium::SdCard => EN.medium_sd_cards,
        }
    }

    /// With its article mid-sentence: "a stick", "an SD card".
    pub const fn a(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_a_stick_inline,
            Medium::SdCard => EN.medium_an_sd_card_inline,
        }
    }

    /// The noun starting a label or sentence: "Stick", "SD card".
    pub const fn cap(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_stick,
            Medium::SdCard => EN.medium_sd_card,
        }
    }

    /// The plural starting a label or sentence: "Sticks", "SD cards".
    pub const fn caps(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_sticks,
            Medium::SdCard => EN.medium_sd_cards,
        }
    }

    /// With its article starting a sentence: "A stick", "An SD card".
    pub const fn a_cap(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_a_stick,
            Medium::SdCard => EN.medium_an_sd_card,
        }
    }

    /// Its full name mid-sentence, where the kind matters: "USB stick",
    /// "SD card".
    pub const fn full(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_usb_stick,
            Medium::SdCard => EN.medium_sd_card,
        }
    }

    /// Its full name with its article, starting a sentence: "A USB
    /// stick", "An SD card".
    pub const fn a_full_cap(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_a_usb_stick,
            Medium::SdCard => EN.medium_an_sd_card,
        }
    }

    /// The half of Files that holds what was copied in, as a label:
    /// "From the stick", "From the SD card".
    pub const fn from_box(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_from_the_stick,
            Medium::SdCard => EN.medium_from_the_sd_card,
        }
    }

    /// The visit's title for the files the stick holds: "On the stick",
    /// "On the SD card".
    pub const fn on_box(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_on_box_stick,
            Medium::SdCard => EN.medium_on_box_sd_card,
        }
    }

    /// An Inbox file's tag when the stick on show holds it: "On this
    /// stick", "On this SD card".
    pub const fn on_this_box(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_on_this_stick,
            Medium::SdCard => EN.medium_on_this_sd_card,
        }
    }

    /// The half of Files that holds what waits to be written, as a
    /// label: "For the stick", "For the SD card".
    pub const fn for_box(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_for_the_stick,
            Medium::SdCard => EN.medium_for_the_sd_card,
        }
    }

    /// Mid-sentence, where a file came from: "from the stick", "from
    /// the SD card".
    pub const fn from_the(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_from_the_stick_inline,
            Medium::SdCard => EN.medium_from_the_sd,
        }
    }

    /// Mid-sentence, where a file already is: "on the stick", "on the SD
    /// card".
    pub const fn on_the(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_on_the_stick,
            Medium::SdCard => EN.medium_on_the_sd_card,
        }
    }

    /// Mid-sentence, where a file waits to go: "for the stick", "for the
    /// SD card".
    pub const fn for_the(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_for_the_stick_inline,
            Medium::SdCard => EN.medium_for_the_sd_card_inline,
        }
    }

    /// The screen that writes what waits for it and copies files in:
    /// "Stick visit", "SD card visit".
    pub const fn visit(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_stick_visit,
            Medium::SdCard => EN.medium_sd_card_visit,
        }
    }

    /// What the medium the device started from is called.
    pub const fn boot(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_boot_stick,
            Medium::SdCard => EN.medium_boot_sd_card,
        }
    }

    /// Its glyph: the USB stick, or the SD card (`docs/BACKUP-FLOW.md`
    /// §17.17).
    pub const fn icon(self) -> Glyph {
        match self {
            Medium::Stick => Glyph::Stick,
            Medium::SdCard => Glyph::Icon(Icon::SdCard),
        }
    }
}
