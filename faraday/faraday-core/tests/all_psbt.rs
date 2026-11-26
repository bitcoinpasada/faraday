//! The integration tests of faraday-core, the psbt area: one module per subject.

#[path = "all/common.rs"]
mod common;

#[path = "all/address_list_whole.rs"]
mod address_list_whole;
#[path = "all/address_look.rs"]
mod address_look;
#[path = "all/address_page.rs"]
mod address_page;
#[path = "all/address_sign.rs"]
mod address_sign;
#[path = "all/address_typed_whole.rs"]
mod address_typed_whole;
#[path = "all/blinding.rs"]
mod blinding;
#[path = "all/bsms.rs"]
mod bsms;
#[path = "all/choices_address_whole.rs"]
mod choices_address_whole;
#[path = "all/collect_leaf_not_run.rs"]
mod collect_leaf_not_run;
#[path = "all/compare.rs"]
mod compare;
#[path = "all/core_wallet.rs"]
mod core_wallet;
#[path = "all/core_wallet_pinned.rs"]
mod core_wallet_pinned;
#[path = "all/core_wallet_tool.rs"]
mod core_wallet_tool;
#[path = "all/create_musig.rs"]
mod create_musig;
#[path = "all/create_on_card.rs"]
mod create_on_card;
#[path = "all/create_vault.rs"]
mod create_vault;
#[path = "all/explore.rs"]
mod explore;
#[path = "all/explore_addresses.rs"]
mod explore_addresses;
#[path = "all/explore_make.rs"]
mod explore_make;
#[path = "all/fee_bump_review.rs"]
mod fee_bump_review;
#[path = "all/gpg_restore.rs"]
mod gpg_restore;
#[path = "all/interop.rs"]
mod interop;
#[path = "all/interop_outcomes.rs"]
mod interop_outcomes;
#[path = "all/invoice.rs"]
mod invoice;
#[path = "all/kef.rs"]
mod kef;
#[path = "all/kef_form.rs"]
mod kef_form;
#[path = "all/kef_make.rs"]
mod kef_make;
#[path = "all/kef_step.rs"]
mod kef_step;
#[path = "all/kef_take_away.rs"]
mod kef_take_away;
#[path = "all/kef_tool.rs"]
mod kef_tool;
#[path = "all/kef_tool_pick.rs"]
mod kef_tool_pick;
#[path = "all/key_checks.rs"]
mod key_checks;
#[path = "all/key_checks_chart.rs"]
mod key_checks_chart;
#[path = "all/key_checks_shown.rs"]
mod key_checks_shown;
#[path = "all/key_path.rs"]
mod key_path;
#[path = "all/list_detail.rs"]
mod list_detail;
#[path = "all/loose_key.rs"]
mod loose_key;
#[path = "all/message_address_row.rs"]
mod message_address_row;
#[path = "all/message_format.rs"]
mod message_format;
#[path = "all/message_held.rs"]
mod message_held;
#[path = "all/message_pick.rs"]
mod message_pick;
#[path = "all/message_scan.rs"]
mod message_scan;
#[path = "all/message_take_away.rs"]
mod message_take_away;
#[path = "all/message_whole.rs"]
mod message_whole;
#[path = "all/multisig_check.rs"]
mod multisig_check;
#[path = "all/names_kept.rs"]
mod names_kept;
#[path = "all/network.rs"]
mod network;
#[path = "all/nonce_check.rs"]
mod nonce_check;
#[path = "all/ots_check.rs"]
mod ots_check;
#[path = "all/proprietary_records.rs"]
mod proprietary_records;
#[path = "all/psbt_v2.rs"]
mod psbt_v2;
#[path = "all/quorum_all_keys.rs"]
mod quorum_all_keys;
#[path = "all/register_wallet.rs"]
mod register_wallet;
#[path = "all/remove_keys.rs"]
mod remove_keys;
#[path = "all/review_address.rs"]
mod review_address;
#[path = "all/review_locks.rs"]
mod review_locks;
#[path = "all/review_stops.rs"]
mod review_stops;
#[path = "all/row_details.rs"]
mod row_details;
#[path = "all/scan.rs"]
mod scan;
#[path = "all/scan_kinds.rs"]
mod scan_kinds;
#[path = "all/scan_pane.rs"]
mod scan_pane;
#[path = "all/scan_viewfinder.rs"]
mod scan_viewfinder;
#[path = "all/shared_keys.rs"]
mod shared_keys;
#[path = "all/sign_from_card.rs"]
mod sign_from_card;
#[path = "all/signer_count.rs"]
mod signer_count;
#[path = "all/silent.rs"]
mod silent;
#[path = "all/silent_hold.rs"]
mod silent_hold;
#[path = "all/spend_backup.rs"]
mod spend_backup;
#[path = "all/spend_from_vault.rs"]
mod spend_from_vault;
#[path = "all/spend_loaded.rs"]
mod spend_loaded;
#[path = "all/spend_sign_card_loads_key.rs"]
mod spend_sign_card_loads_key;
#[path = "all/spend_wallet_removed.rs"]
mod spend_wallet_removed;
#[path = "all/sweep.rs"]
mod sweep;
#[path = "all/totp.rs"]
mod totp;
#[path = "all/vanity.rs"]
mod vanity;
#[path = "all/wallet_confirm.rs"]
mod wallet_confirm;
#[path = "all/wallet_files_other.rs"]
mod wallet_files_other;
#[path = "all/wallet_id.rs"]
mod wallet_id;
#[path = "all/wallet_names.rs"]
mod wallet_names;
#[path = "all/wallet_not_your_key.rs"]
mod wallet_not_your_key;
#[path = "all/wallet_secret_text.rs"]
mod wallet_secret_text;
#[path = "all/wallets_page.rs"]
mod wallets_page;
