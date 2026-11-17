//! The integration tests of faraday-core, the codex32 area: one module per subject.

#[path = "all/common.rs"]
mod common;

#[path = "all/bip85.rs"]
mod bip85;
#[path = "all/bip85_child.rs"]
mod bip85_child;
#[path = "all/codex32_correct.rs"]
mod codex32_correct;
#[path = "all/codex32_dice.rs"]
mod codex32_dice;
#[path = "all/codex32_forms.rs"]
mod codex32_forms;
#[path = "all/codex32_glyphs.rs"]
mod codex32_glyphs;
#[path = "all/codex32_guide.rs"]
mod codex32_guide;
#[path = "all/codex32_input_panel.rs"]
mod codex32_input_panel;
#[path = "all/codex32_rest.rs"]
mod codex32_rest;
#[path = "all/codex32_roller.rs"]
mod codex32_roller;
#[path = "all/codex32_tabs.rs"]
mod codex32_tabs;
#[path = "all/codex32qr_copy.rs"]
mod codex32qr_copy;
#[path = "all/convert.rs"]
mod convert;
#[path = "all/dice_input_panel.rs"]
mod dice_input_panel;
#[path = "all/forms.rs"]
mod forms;
#[path = "all/import.rs"]
mod import;
#[path = "all/keygen_slip39.rs"]
mod keygen_slip39;
#[path = "all/passphrase_paper.rs"]
mod passphrase_paper;
#[path = "all/paste_a_seed.rs"]
mod paste_a_seed;
#[path = "all/published_words.rs"]
mod published_words;
#[path = "all/restore.rs"]
mod restore;
#[path = "all/restore_shares.rs"]
mod restore_shares;
#[path = "all/roller.rs"]
mod roller;
#[path = "all/scan_a_key.rs"]
mod scan_a_key;
#[path = "all/seed_backup.rs"]
mod seed_backup;
#[path = "all/seed_check.rs"]
mod seed_check;
#[path = "all/seeds_first.rs"]
mod seeds_first;
#[path = "all/word_lists.rs"]
mod word_lists;
