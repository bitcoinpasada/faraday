//! What the removable medium is called. The PC's is a stick (a USB stick,
//! or an SD card in a USB reader); the Pi reads its own SD card and USB
//! sticks alike, so it says "card or stick". The shell says which at start, and every
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
    /// An SD card or a USB stick, read together: the Pi.
    SdCard,
}

impl Medium {
    /// The noun mid-sentence: "stick", "card or stick".
    pub const fn noun(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_stick_inline,
            Medium::SdCard => EN.medium_card_or_stick_inline,
        }
    }

    /// The plural mid-sentence: "sticks", "cards or sticks".
    pub const fn nouns(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_sticks_inline,
            Medium::SdCard => EN.medium_cards_or_sticks_inline,
        }
    }

    /// With its article mid-sentence: "a stick", "a card or stick".
    pub const fn a(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_a_stick_inline,
            Medium::SdCard => EN.medium_a_card_or_stick_inline,
        }
    }

    /// The noun starting a label or sentence: "Stick", "Card or stick".
    pub const fn cap(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_stick,
            Medium::SdCard => EN.medium_card_or_stick,
        }
    }

    /// The plural starting a label or sentence: "Sticks", "Cards or sticks".
    pub const fn caps(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_sticks,
            Medium::SdCard => EN.medium_cards_or_sticks,
        }
    }

    /// With its article starting a sentence: "A stick", "A card or stick".
    pub const fn a_cap(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_a_stick,
            Medium::SdCard => EN.medium_a_card_or_stick,
        }
    }

    /// Its full name mid-sentence, where the kind matters: "USB stick",
    /// "SD card or USB stick".
    pub const fn full(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_usb_stick,
            Medium::SdCard => EN.medium_sd_card_or_usb_stick,
        }
    }

    /// Its full name with its article, starting a sentence: "A USB
    /// stick", "An SD card or USB stick".
    pub const fn a_full_cap(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_a_usb_stick,
            Medium::SdCard => EN.medium_an_sd_card_or_usb_stick,
        }
    }

    /// The half of Files that holds what was copied in, as a label:
    /// "From the stick", "From the card or stick".
    pub const fn from_box(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_from_the_stick,
            Medium::SdCard => EN.medium_from_the_card_or_stick,
        }
    }

    /// The visit's title for the files the stick holds: "On the stick",
    /// "On the card or stick".
    pub const fn on_box(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_on_box_stick,
            Medium::SdCard => EN.medium_on_box_card_or_stick,
        }
    }

    /// An Inbox file's tag when the stick on show holds it: "On this
    /// stick", "On this card or stick".
    pub const fn on_this_box(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_on_this_stick,
            Medium::SdCard => EN.medium_on_this_card_or_stick,
        }
    }

    /// The half of Files that holds what waits to be written, as a
    /// label: "For the stick", "For the card or stick".
    pub const fn for_box(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_for_the_stick,
            Medium::SdCard => EN.medium_for_the_card_or_stick,
        }
    }

    /// Mid-sentence, where a file came from: "from the stick", "from
    /// the card or stick".
    pub const fn from_the(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_from_the_stick_inline,
            Medium::SdCard => EN.medium_from_the_card_or_stick_inline,
        }
    }

    /// Mid-sentence, where a file already is: "on the stick", "on the
    /// card or stick".
    pub const fn on_the(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_on_the_stick,
            Medium::SdCard => EN.medium_on_the_card_or_stick,
        }
    }

    /// Mid-sentence, where a file waits to go: "for the stick", "for the
    /// card or stick".
    pub const fn for_the(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_for_the_stick_inline,
            Medium::SdCard => EN.medium_for_the_card_or_stick_inline,
        }
    }

    /// The card that lays the vaults on the media: "Sticks and vaults",
    /// "Cards, sticks and vaults".
    pub const fn and_vaults(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_sticks_and_vaults,
            Medium::SdCard => EN.medium_cards_or_sticks_and_vaults,
        }
    }

    /// The card that brings the media in: "Bring in the sticks".
    pub const fn bring_in(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_bring_in_sticks,
            Medium::SdCard => EN.medium_bring_in_cards_or_sticks,
        }
    }

    /// The card that writes the media: "Write the sticks".
    pub const fn write_the(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_write_the_sticks,
            Medium::SdCard => EN.medium_write_the_cards_or_sticks,
        }
    }

    /// The checklist item that writes to the media: "Write to sticks".
    pub const fn write_to(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_write_to_sticks,
            Medium::SdCard => EN.medium_write_to_cards_or_sticks,
        }
    }

    /// What the medium the device started from is called.
    pub const fn boot(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_boot_stick,
            Medium::SdCard => EN.medium_boot_sd_card,
        }
    }

    /// What offers the stick the device started from: "Use the boot
    /// stick", "Use the boot SD card".
    pub const fn use_boot(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_use_boot_stick,
            Medium::SdCard => EN.medium_use_boot_sd_card,
        }
    }

    /// The medium the device starts from, mid-sentence: "stick", "SD card".
    /// A Pi starts only from its SD card, so it never says "card or stick"
    /// where the sentence is about starting the machine.
    pub const fn started_from(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_stick_inline,
            Medium::SdCard => EN.medium_started_from_sd_card,
        }
    }

    /// As `started_from`, by its full name: "USB stick", "SD card".
    pub const fn started_from_full(self) -> &'static str {
        match self {
            Medium::Stick => EN.medium_usb_stick,
            Medium::SdCard => EN.medium_started_from_sd_card,
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
