//! Faraday's English words for the medium area.

macro_rules! words_medium {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- medium (faraday-core/src/medium.rs) -----
    /// Text in `noun`.
    medium_stick_inline: "stick",
    /// Text in `nouns`.
    medium_sticks_inline: "sticks",
    /// Text in `nouns`.
    medium_cards_or_sticks_inline: "cards or sticks",
    /// Text in `a`.
    medium_a_stick_inline: "a stick",
    /// Text in `a`.
    medium_a_card_or_stick_inline: "a card or stick",
    /// Text in `noun`.
    medium_card_or_stick_inline: "card or stick",
    /// Text in `cap`.
    medium_stick: "Stick",
    /// Text in `cap`.
    medium_card_or_stick: "Card or stick",
    /// Text in `caps`.
    medium_sticks: "Sticks",
    /// Text in `caps`.
    medium_cards_or_sticks: "Cards or sticks",
    /// Text in `a_cap`.
    medium_a_stick: "A stick",
    /// Text in `a_cap`.
    medium_a_card_or_stick: "A card or stick",
    /// Text in `full`.
    medium_usb_stick: "USB stick",
    /// Text in `full`.
    medium_sd_card_or_usb_stick: "SD card or USB stick",
    /// Text in `a_full_cap`.
    medium_a_usb_stick: "A USB stick",
    /// Text in `a_full_cap`.
    medium_an_sd_card_or_usb_stick: "An SD card or USB stick",
    /// Text in `from_box`.
    medium_from_the_stick: "From the stick",
    /// Text in `from_box`.
    medium_from_the_card_or_stick: "From the card or stick",
    /// Text in `for_box`.
    medium_for_the_stick: "For the stick",
    /// Text in `for_box`.
    medium_for_the_card_or_stick: "For the card or stick",
    /// Text in `on_box`.
    medium_on_box_stick: "On the stick",
    /// Text in `on_box`.
    medium_on_box_card_or_stick: "On the card or stick",
    /// Text in `on_this_box`.
    medium_on_this_stick: "On this stick",
    /// Text in `on_this_box`.
    medium_on_this_card_or_stick: "On this card or stick",
    /// Text in `from_the`.
    medium_from_the_stick_inline: "from the stick",
    /// Text in `from_the`.
    medium_from_the_card_or_stick_inline: "from the card or stick",
    /// Text in `on_the`.
    medium_on_the_stick: "on the stick",
    /// Text in `on_the`.
    medium_on_the_card_or_stick: "on the card or stick",
    /// Text in `to_write_to`.
    medium_to_write_to_the_stick_inline: "to write to the stick",
    /// Text in `to_write_to`.
    medium_to_write_to_the_card_or_stick_inline: "to write to the card or stick",
    /// Text in `for_the`.
    medium_for_the_stick_inline: "for the stick",
    /// Text in `for_the`.
    medium_for_the_card_or_stick_inline: "for the card or stick",
    /// Text in `and_vaults`.
    medium_sticks_and_vaults: "Sticks and vaults",
    /// Text in `and_vaults`.
    medium_cards_or_sticks_and_vaults: "Cards, sticks and vaults",
    /// The way back's name for the checklist item of blank templates.
    backup_step_templates: "Blank templates",
    /// The way back's name for the checklist item of a seed copied by hand.
    backup_step_copy: "Copy by hand",
    /// The way back's name for the checklist item of codex32 strings.
    backup_step_codex32: "Codex32 strings",
    /// The way back's name for the checklist item of Seed XOR parts.
    backup_step_parts: "Seed XOR parts",
    /// The way back's name for the checklist item of a passphrase on paper.
    backup_step_passphrase: "Passphrase on paper",
    /// The way back's name for the checklist item of a Krux code.
    backup_step_kef: "Krux code",
    /// The way back's name for the checklist item of a vault.
    backup_step_vault: "Vault",
    /// Text in `bring_in`: step 2's card of the backup flow and of the setup of a wallet's vaults.
    medium_bring_in_sticks: "Bring in the sticks",
    /// Text in `bring_in`.
    medium_bring_in_cards_or_sticks: "Bring in the cards or sticks",
    /// Text in `write_the`: step 6's card of the backup flow.
    medium_write_the_sticks: "Write the sticks",
    /// Text in `write_the`.
    medium_write_the_cards_or_sticks: "Write the cards or sticks",
    /// Text in `write_to`: the checklist item that writes to the sticks, by place.
    medium_write_to_sticks: "Write to sticks",
    /// Text in `write_to`.
    medium_write_to_cards_or_sticks: "Write to cards or sticks",
    /// Text in `boot`.
    medium_boot_stick: "Boot stick",
    /// Text in `boot`.
    medium_boot_sd_card: "Boot SD card",
    /// Text in `started_from`, `started_from_full`.
    medium_started_from_sd_card: "SD card",
    /// Text in `use_boot`.
    medium_use_boot_stick: "Use the boot stick",
    /// Text in `use_boot`.
    medium_use_boot_sd_card: "Use the boot SD card",
        }
    };
}
