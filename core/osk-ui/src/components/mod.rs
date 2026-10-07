//! The content-kind components (`docs/DESIGN.md` §4).
//!
//! §4 is an inventory of content kinds with one rule each. This module
//! has one function per kind, in one file per §4 group, and each function
//! quotes the rule it implements. A screen picks a component; it never
//! picks a font, a row height or a mask policy of its own, which is what
//! "one kind of content, one rendering" (§2.2) means in code.
//!
//! Every number these functions use is a name from [`crate::tokens`];
//! `tools/lint-tokens.sh` fails the build when one is written here
//! instead. Interaction reuses the hit targets that already exist
//! ([`crate::widgets::HitTarget`]), so nothing here needs new state.
//!
//! [`crate::organisms`] still holds the older composites the application
//! screens are built from. They are marked where a component replaces
//! them and go when the screens move.

pub mod actions;
pub mod amounts;
pub mod badges;
pub mod choice;
pub mod codes;
pub mod entry;
pub mod identity;
pub mod nav;
pub mod paths;
pub mod progress;
pub mod records;
pub mod secrets;
pub mod strings;
pub mod text;

pub use actions::{hold, primary, row_action, secondary};
pub use amounts::{Denomination, Unit, amount, denominated, fee_rows, rate};
pub use badges::{
    Network, OutputBadge, caution_badge, network_badge, output_badge, state_row, tier_badge,
};
pub use choice::{
    choice_caution_row, choice_list, choice_row, choice_value_row, fact_row, fingerprint_row,
    flat_fact_row, key_choice_row, mono_choice_row, mono_value_row, pair, preset_row, setting_list,
    setting_row, toggle_row, value_row,
};
pub use codes::{
    Preview, animated_row, code_label, progress_bar, qr_block, qr_grid, qr_grid_pages,
    qr_grid_region, qr_grid_span, qr_with_label, viewfinder, viewfinder_side,
};
pub use entry::{
    Entries, Strip, candidate_strip, coin_pad, dice_pad, entropy_entries, entropy_progress,
    entropy_progress_height, inline_caption, inline_error, pad_group, pin_pad, text_field,
};
pub use identity::{
    fingerprint, fingerprint_glyph, flat_key_context, key_chips, key_context, key_row, sign_with,
};
pub use nav::{
    Tile, app_bar, bar_action, bar_eye, bar_info, dimmed_row, hub, menu_row, menu_row_mono, pager,
    pager_mono, sidebar, status_line,
};
pub use paths::{path, path_row, script_type_row};
pub use progress::{empty_row, progress, progress_of, reveal_remaining, terminal};
pub use records::{Record, record, record_height, result, warning_card, warning_card_height};
pub use secrets::{Secret, secret_panel, secret_row, words_so_far};
pub use strings::{
    comparison_string, descriptor, elide, flat_reference_row, masked_comparison_string,
    reference_row,
};
pub use text::{explainer, fingerprint_value, label, reason, title, value};
