//! The integration tests of faraday-core, the backup area: one module per subject.

#[path = "all/common.rs"]
mod common;

#[path = "all/ascii_passphrase.rs"]
mod ascii_passphrase;
#[path = "all/backup_again.rs"]
mod backup_again;
#[path = "all/backup_ages.rs"]
mod backup_ages;
#[path = "all/backup_board.rs"]
mod backup_board;
#[path = "all/backup_board_vaults.rs"]
mod backup_board_vaults;
#[path = "all/backup_checklist.rs"]
mod backup_checklist;
#[path = "all/backup_first.rs"]
mod backup_first;
#[path = "all/backup_flow.rs"]
mod backup_flow;
#[path = "all/backup_flow_wallet.rs"]
mod backup_flow_wallet;
#[path = "all/backup_flow_wallet_late.rs"]
mod backup_flow_wallet_late;
#[path = "all/backup_flow_wallet_moved.rs"]
mod backup_flow_wallet_moved;
#[path = "all/backup_hold_passphrases.rs"]
mod backup_hold_passphrases;
#[path = "all/backup_hold_vault_passphrase.rs"]
mod backup_hold_vault_passphrase;
#[path = "all/backup_keep.rs"]
mod backup_keep;
#[path = "all/backup_marks.rs"]
mod backup_marks;
#[path = "all/backup_plan.rs"]
mod backup_plan;
#[path = "all/backup_public_stick.rs"]
mod backup_public_stick;
#[path = "all/backup_read_back.rs"]
mod backup_read_back;
#[path = "all/backup_restores.rs"]
mod backup_restores;
#[path = "all/backup_sets.rs"]
mod backup_sets;
#[path = "all/backup_spend_reach.rs"]
mod backup_spend_reach;
#[path = "all/backup_stick.rs"]
mod backup_stick;
#[path = "all/backup_which.rs"]
mod backup_which;
#[path = "all/backups.rs"]
mod backups;
#[path = "all/boot_import.rs"]
mod boot_import;
#[path = "all/chart_edit.rs"]
mod chart_edit;
#[path = "all/codex32_backup.rs"]
mod codex32_backup;
#[path = "all/codex32_kept.rs"]
mod codex32_kept;
#[path = "all/codex32_left.rs"]
mod codex32_left;
#[path = "all/codex32_make.rs"]
mod codex32_make;
#[path = "all/codex32_round.rs"]
mod codex32_round;
#[path = "all/codex32_vault_set.rs"]
mod codex32_vault_set;
#[path = "all/create_shares.rs"]
mod create_shares;
#[path = "all/create_wallet.rs"]
mod create_wallet;
#[path = "all/fills_read.rs"]
mod fills_read;
#[path = "all/gpg.rs"]
mod gpg;
#[path = "all/icon_tooltips.rs"]
mod icon_tooltips;
#[path = "all/kef_scan.rs"]
mod kef_scan;
#[path = "all/keygen.rs"]
mod keygen;
#[path = "all/keygen_steps.rs"]
mod keygen_steps;
#[path = "all/lightning.rs"]
mod lightning;
#[path = "all/made_online.rs"]
mod made_online;
#[path = "all/own_randomness.rs"]
mod own_randomness;
#[path = "all/paper.rs"]
mod paper;
#[path = "all/pass_fields.rs"]
mod pass_fields;
#[path = "all/passphrase_kept.rs"]
mod passphrase_kept;
#[path = "all/plan_fits.rs"]
mod plan_fits;
#[path = "all/rounds.rs"]
mod rounds;
#[path = "all/scan_lock.rs"]
mod scan_lock;
#[path = "all/steps_hidden.rs"]
mod steps_hidden;
#[path = "all/stick_pills.rs"]
mod stick_pills;
#[path = "all/vault_load_job.rs"]
mod vault_load_job;
#[path = "all/vault_memory.rs"]
mod vault_memory;
#[path = "all/vault_seal.rs"]
mod vault_seal;
#[path = "all/vault_seal_count.rs"]
mod vault_seal_count;
#[path = "all/vault_seal_stop.rs"]
mod vault_seal_stop;
#[path = "all/wallet_id_plans.rs"]
mod wallet_id_plans;
#[path = "all/wallet_id_summaries.rs"]
mod wallet_id_summaries;
#[path = "all/wallet_id_vault.rs"]
mod wallet_id_vault;
#[path = "all/wallet_paths.rs"]
mod wallet_paths;
#[path = "all/wallet_setup.rs"]
mod wallet_setup;
#[path = "all/wide_board.rs"]
mod wide_board;
#[path = "all/worker.rs"]
mod worker;
