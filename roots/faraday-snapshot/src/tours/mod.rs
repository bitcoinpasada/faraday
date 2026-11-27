//! The tours, one module each, and the one table that lists them.

use crate::runner::Tour;

mod addkey;
mod again;
mod band;
mod board;
mod boardunsorted;
mod boot;
mod bootplace;
mod busy;
mod cancel;
mod chartedit;
mod check;
mod check_address;
mod codex32;
mod codex32backup;
mod codex32correct;
mod codex32form;
mod compact;
mod components;
mod convert;
mod copy;
mod core_wallet;
mod createfrom;
mod createshares;
mod createslots;
mod default;
mod demo;
mod devices;
mod dice5;
mod dicewords;
mod files_wide;
mod first;
mod flipwords;
mod flow;
mod flowbeside;
mod flowguide;
mod fromvault;
mod glance;
mod gpg_restore;
mod home_search;
mod host;
mod how;
mod inbox;
mod inprogress;
mod invoice;
mod kef;
mod kef_make;
mod kef_menu;
mod kef_step;
mod kef_ways;
mod kept;
mod keygen;
mod keypath;
mod keys;
mod lightning_cln;
mod m3;
mod menu;
mod modes;
mod multisig_check;
mod nonce;
mod oldparts;
mod open_tools;
mod osk;
mod ots;
mod panes;
mod parts;
mod passseeds;
mod pills;
mod plan;
mod power;
mod public;
mod qr;
mod quick;
mod randomness;
mod register;
mod restore;
mod roller;
mod rowdetails;
mod scan;
mod seed_pick;
mod seedfile;
mod seeds;
mod signer;
mod silent;
mod site_tour;
mod spend;
mod spend_pane;
mod standard;
mod themes;
mod toasts;
mod tooltips;
mod transfer;
mod vaultsheets;
mod vaultshow;
mod vaultspair;
mod vaultstates;
mod vaultway;
mod visit;
mod walletflow;

/// A tour: a scripted walk over one run.
pub type TourFn = fn(&mut Tour) -> Result<(), String>;

/// Every tour run by name, in the order a command that runs them all runs them: the name and
/// the tour's function. `default` and `boot` are not in it; the runner calls them.
pub const TOURS: &[(&str, TourFn)] = &[
    ("spend", spend::run),
    ("check", check::run),
    ("check-address", check_address::run),
    ("open-tools", open_tools::run),
    ("rowdetails", rowdetails::run),
    ("band", band::run),
    ("convert", convert::run),
    ("seed-pick", seed_pick::run),
    ("invoice", invoice::run),
    ("devices", devices::run),
    ("toasts", toasts::run),
    ("themes", themes::run),
    ("tooltips", tooltips::run),
    ("compact", compact::run),
    ("home-search", home_search::run),
    ("seeds", seeds::run),
    ("keys", keys::run),
    ("visit", visit::run),
    ("files-wide", files_wide::run),
    ("copy", copy::run),
    ("scan", scan::run),
    ("public", public::run),
    ("qr", qr::run),
    ("lightning-cln", lightning_cln::run),
    ("seedfile", seedfile::run),
    ("vaultway", vaultway::run),
    ("vaultstates", vaultstates::run),
    ("kef", kef::run),
    ("kef-menu", kef_menu::run),
    ("kef-ways", kef_ways::run),
    ("kef-make", kef_make::run),
    ("kef-step", kef_step::run),
    ("ots", ots::run),
    ("core-wallet", core_wallet::run),
    ("kept", kept::run),
    ("again", again::run),
    ("quick", quick::run),
    ("plan", plan::run),
    ("board", board::run),
    ("boardunsorted", boardunsorted::run),
    ("menu", menu::run),
    ("m3", m3::run),
    ("inbox", inbox::run),
    ("silent", silent::run),
    ("keypath", keypath::run),
    ("register", register::run),
    ("multisig-check", multisig_check::run),
    ("nonce", nonce::run),
    ("spend-pane", spend_pane::run),
    ("signer", signer::run),
    ("transfer", transfer::run),
    ("glance", glance::run),
    ("chartedit", chartedit::run),
    ("oldparts", oldparts::run),
    ("fromvault", fromvault::run),
    ("first", first::run),
    ("restore", restore::run),
    ("osk", osk::run),
    ("flow", flow::run),
    ("flowbeside", flowbeside::run),
    ("flowguide", flowguide::run),
    ("gpg-restore", gpg_restore::run),
    ("bootplace", bootplace::run),
    ("passseeds", passseeds::run),
    ("power", power::run),
    ("pills", pills::run),
    ("components", components::run),
    ("host", host::run),
    ("walletflow", walletflow::run),
    ("demo", demo::run),
    ("modes", modes::run),
    ("vaultshow", vaultshow::run),
    ("vaultspair", vaultspair::run),
    ("vaultsheets", vaultsheets::run),
    ("addkey", addkey::run),
    ("keygen", keygen::run),
    ("dicewords", dicewords::run),
    ("flipwords", flipwords::run),
    ("codex32correct", codex32correct::run),
    ("createfrom", createfrom::run),
    ("inprogress", inprogress::run),
    ("cancel", cancel::run),
    ("busy", busy::run),
    ("how", how::run),
    ("std", standard::run),
    ("parts", parts::run),
    ("panes", panes::run),
    ("roller", roller::run),
    ("randomness", randomness::run),
    ("dice5", dice5::run),
    ("tour", site_tour::run),
    ("codex32", codex32::run),
    ("codex32form", codex32form::run),
    ("createslots", createslots::run),
    ("createshares", createshares::run),
    ("codex32backup", codex32backup::run),
];

/// The names of the tours, as the usage line lists them.
#[must_use]
pub fn names() -> String {
    todo!()
}

/// The tour named `name`, run; `None` for a name no tour has.
pub fn run_named(_t: &mut Tour, _name: &str) -> Option<Result<(), String>> {
    todo!()
}

/// The tour that runs when no name is given.
pub fn run_default(t: &mut Tour) -> Result<(), String> {
    default::run(t)
}

/// The boot tour.
pub fn run_boot(t: &mut Tour) -> Result<(), String> {
    boot::run(t)
}
