//! The integration tests of faraday-core, the vault area: one module per subject.

#[path = "all/common.rs"]
mod common;

#[path = "all/create_vault_fade.rs"]
mod create_vault_fade;
#[path = "all/flow_hide_shown.rs"]
mod flow_hide_shown;
#[path = "all/kdbx.rs"]
mod kdbx;
#[path = "all/mode_files.rs"]
mod mode_files;
#[path = "all/save_without_encryption.rs"]
mod save_without_encryption;
#[path = "all/sd_card.rs"]
mod sd_card;
#[path = "all/settings_file.rs"]
mod settings_file;
#[path = "all/signer_check.rs"]
mod signer_check;
#[path = "all/stick_copy.rs"]
mod stick_copy;
#[path = "all/stick_with_vault.rs"]
mod stick_with_vault;
#[path = "all/tool_stick.rs"]
mod tool_stick;
#[path = "all/vault_by_contents.rs"]
mod vault_by_contents;
#[path = "all/vault_copy.rs"]
mod vault_copy;
#[path = "all/vault_full.rs"]
mod vault_full;
#[path = "all/vault_key_checks.rs"]
mod vault_key_checks;
#[path = "all/vault_keys.rs"]
mod vault_keys;
#[path = "all/vault_load.rs"]
mod vault_load;
#[path = "all/vault_name.rs"]
mod vault_name;
#[path = "all/vault_passphrase.rs"]
mod vault_passphrase;
#[path = "all/vault_seal_room.rs"]
mod vault_seal_room;
#[path = "all/vault_search.rs"]
mod vault_search;
#[path = "all/vault_settings_line.rs"]
mod vault_settings_line;
#[path = "all/vault_short_window.rs"]
mod vault_short_window;
#[path = "all/vault_show_save.rs"]
mod vault_show_save;
#[path = "all/vault_states.rs"]
mod vault_states;
#[path = "all/vault_summary.rs"]
mod vault_summary;
#[path = "all/vault_tabs.rs"]
mod vault_tabs;
#[path = "all/vault_trim.rs"]
mod vault_trim;
#[path = "all/vault_way.rs"]
mod vault_way;
