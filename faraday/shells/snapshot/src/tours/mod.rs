//! The tours, one file each (a few small ones by subject), and the one
//! table that lists them.

use crate::tour::Tour;
use faraday_crypto::work::Compute;

pub(crate) mod add_key;
pub(crate) mod again;
pub(crate) mod board;
pub(crate) mod boot;
pub(crate) mod boot_place;
pub(crate) mod busy;
pub(crate) mod cancel;
pub(crate) mod chart_edit;
pub(crate) mod codex32;
pub(crate) mod codex32_backup;
pub(crate) mod codex32_correct;
pub(crate) mod codex32_form;
pub(crate) mod compact;
pub(crate) mod components;
pub(crate) mod convert;
pub(crate) mod copy;
pub(crate) mod core_wallet;
pub(crate) mod create_from;
pub(crate) mod create_shares;
pub(crate) mod default;
pub(crate) mod desktop;
pub(crate) mod devices;
pub(crate) mod dice_five;
pub(crate) mod first;
pub(crate) mod flow;
pub(crate) mod from_vault;
pub(crate) mod glance;
pub(crate) mod gpg_restore;
pub(crate) mod guide;
pub(crate) mod host;
pub(crate) mod how;
pub(crate) mod in_progress;
pub(crate) mod inbox;
pub(crate) mod invoice;
pub(crate) mod kef;
pub(crate) mod kept;
pub(crate) mod key_path;
pub(crate) mod keygen;
pub(crate) mod keys;
pub(crate) mod lightning_cln;
pub(crate) mod m3;
pub(crate) mod menu;
pub(crate) mod nonce;
pub(crate) mod ots_check;
pub(crate) mod panes;
pub(crate) mod partials;
pub(crate) mod pass_seeds;
pub(crate) mod plan;
pub(crate) mod public;
pub(crate) mod qr;
pub(crate) mod qr_tool;
pub(crate) mod randomness;
pub(crate) mod register;
pub(crate) mod restore;
pub(crate) mod roller;
pub(crate) mod seedfile;
pub(crate) mod seeds;
pub(crate) mod signer;
pub(crate) mod silent;
pub(crate) mod site_tour;
pub(crate) mod spend;
pub(crate) mod standard;
pub(crate) mod themes;
pub(crate) mod tooltips;
pub(crate) mod transfer;
pub(crate) mod vault_sheets;
pub(crate) mod vault_show;
pub(crate) mod vault_states;
pub(crate) mod vaultway;
pub(crate) mod visit;
pub(crate) mod wallet_flow;

/// A tour's function: the costly ones take the compute token.
pub(crate) enum TourFn {
    Plain(fn(&mut Tour) -> Result<(), String>),
    Costly(fn(&Compute, &mut Tour) -> Result<(), String>),
}

/// Every tour `faraday-snapshot` runs by name, in the order a command
/// that runs them all runs them: the name and the function. The usage
/// line and the dispatch come from this table; what a tour shows is in
/// its file's `//!` doc.
pub(crate) const TOURS: &[(&str, TourFn)] = &[
    ("spend", TourFn::Plain(spend::spend_tour)),
    ("convert", TourFn::Plain(convert::convert_tour)),
    ("invoice", TourFn::Plain(invoice::invoice_tour)),
    ("devices", TourFn::Plain(devices::devices_tour)),
    ("toasts", TourFn::Plain(devices::toasts_tour)),
    ("themes", TourFn::Plain(themes::themes_tour)),
    ("tooltips", TourFn::Plain(tooltips::tooltips_tour)),
    ("compact", TourFn::Plain(compact::compact_tour)),
    ("seeds", TourFn::Plain(seeds::seeds_tour)),
    ("keys", TourFn::Plain(keys::keys_tour)),
    ("visit", TourFn::Costly(visit::visit_tour)),
    ("copy", TourFn::Costly(copy::copy_tour)),
    ("scan", TourFn::Plain(devices::scan_tour)),
    ("public", TourFn::Costly(public::public_tour)),
    ("qr", TourFn::Costly(qr::qr_tour)),
    (
        "lightning-cln",
        TourFn::Plain(lightning_cln::lightning_cln_tour),
    ),
    ("seedfile", TourFn::Costly(seedfile::seedfile_tour)),
    ("vaultway", TourFn::Plain(vaultway::vaultway_tour)),
    (
        "vaultstates",
        TourFn::Plain(vault_states::vault_states_tour),
    ),
    ("kef", TourFn::Plain(kef::kef_tour)),
    ("kef-menu", TourFn::Plain(kef::kef_menu_tour)),
    ("kef-make", TourFn::Costly(kef::kef_make_tour)),
    ("kef-step", TourFn::Plain(kef::kef_step_tour)),
    ("qr-tool", TourFn::Plain(qr_tool::qr_tool_tour)),
    ("ots", TourFn::Plain(ots_check::ots_check_tour)),
    ("core-wallet", TourFn::Plain(core_wallet::core_wallet_tour)),
    ("kept", TourFn::Costly(kept::kept_tour)),
    ("again", TourFn::Plain(again::again_tour)),
    ("plan", TourFn::Costly(plan::plan_tour)),
    ("board", TourFn::Costly(board::board_tour)),
    ("boardunsorted", TourFn::Costly(board::board_unsorted_tour)),
    ("menu", TourFn::Plain(menu::menu_tour)),
    ("m3", TourFn::Plain(m3::m3_tour)),
    ("inbox", TourFn::Plain(inbox::inbox_tour)),
    ("silent", TourFn::Costly(silent::silent_tour)),
    ("keypath", TourFn::Plain(key_path::key_path_tour)),
    ("register", TourFn::Plain(register::register_tour)),
    ("nonce", TourFn::Plain(nonce::nonce_tour)),
    ("signer", TourFn::Plain(signer::signer_tour)),
    ("transfer", TourFn::Plain(transfer::transfer_tour)),
    ("glance", TourFn::Costly(glance::glance_tour)),
    ("chartedit", TourFn::Costly(chart_edit::chart_edit_tour)),
    ("fromvault", TourFn::Costly(from_vault::from_vault_tour)),
    ("first", TourFn::Costly(first::first_tour)),
    ("restore", TourFn::Plain(restore::restore_tour)),
    ("osk", TourFn::Plain(devices::osk_tour)),
    ("flow", TourFn::Plain(flow::flow_tour)),
    ("flowbeside", TourFn::Plain(flow::flow_beside_tour)),
    ("gpg-restore", TourFn::Plain(gpg_restore::gpg_restore_tour)),
    ("bootplace", TourFn::Plain(boot_place::boot_place_tour)),
    ("passseeds", TourFn::Costly(pass_seeds::pass_seeds_tour)),
    ("power", TourFn::Plain(devices::power_tour)),
    ("pills", TourFn::Plain(desktop::pills_tour)),
    ("components", TourFn::Plain(components::components_tour)),
    ("host", TourFn::Plain(host::host_tour)),
    ("walletflow", TourFn::Costly(wallet_flow::wallet_flow_tour)),
    ("demo", TourFn::Plain(desktop::demo_tour)),
    ("modes", TourFn::Plain(desktop::modes_tour)),
    ("vaultshow", TourFn::Plain(vault_show::vault_show_tour)),
    (
        "vaultsheets",
        TourFn::Costly(vault_sheets::vault_sheets_tour),
    ),
    ("addkey", TourFn::Plain(add_key::add_key_tour)),
    ("keygen", TourFn::Plain(keygen::keygen_tour)),
    (
        "codex32correct",
        TourFn::Costly(codex32_correct::codex32_correct_tour),
    ),
    ("createfrom", TourFn::Costly(create_from::create_from_tour)),
    ("inprogress", TourFn::Plain(in_progress::in_progress_tour)),
    ("cancel", TourFn::Plain(cancel::cancel_tour)),
    ("busy", TourFn::Plain(busy::busy_tour)),
    ("how", TourFn::Plain(how::how_tour)),
    ("std", TourFn::Plain(standard::standard_tour)),
    ("parts", TourFn::Plain(partials::partials_tour)),
    ("panes", TourFn::Plain(panes::panes_tour)),
    ("roller", TourFn::Plain(roller::roller_tour)),
    ("randomness", TourFn::Plain(randomness::randomness_tour)),
    ("dice5", TourFn::Plain(dice_five::dice_five_tour)),
    ("tour", TourFn::Costly(site_tour::site_tour)),
    ("codex32", TourFn::Plain(codex32::codex32_tour)),
    ("codex32tour", TourFn::Plain(codex32::codex32_shares_tour)),
    (
        "codex32form",
        TourFn::Plain(codex32_form::codex32_form_tour),
    ),
    (
        "createshares",
        TourFn::Plain(create_shares::create_shares_tour),
    ),
    (
        "codex32backup",
        TourFn::Plain(codex32_backup::codex32_backup_tour),
    ),
];

/// The names of the tours, as the usage line lists them.
pub(crate) fn names() -> String {
    TOURS.iter().map(|(n, _)| *n).collect::<Vec<_>>().join("|")
}

/// Costs what the tour it runs costs. The tour named `name`, run;
/// `None` for a name no tour has.
pub(crate) fn run_named(c: &Compute, t: &mut Tour, name: &str) -> Option<Result<(), String>> {
    let (_, f) = TOURS.iter().find(|(n, _)| *n == name)?;
    Some(match f {
        TourFn::Plain(f) => f(t),
        TourFn::Costly(f) => f(c, t),
    })
}
