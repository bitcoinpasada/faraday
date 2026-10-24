//! Renders Faraday's screens to PNG along one scripted tour.
//!
//! ```text
//! faraday-snapshot [--sd-card] [--network mainnet] WxH[@DPI] TESTKIT_DIR OUT_DIR [spend|themes|compact|seeds|keys|visit|copy|scan|public|seedfile|vaultway|kept|again|plan|board|inbox|transfer|upgrade|glance|chartedit|fromvault|first|restore|osk|flow|bootplace|passseeds|power|pills|guide|tour|codex32]
//! ```
//!
//! `--sd-card` runs the tour as the Pi's stick shell starts the app:
//! with the removable medium an SD card (`Medium::SdCard`), so every
//! screen names it as the Pi does.
//!
//! `--network mainnet` runs the tour as a Mainnet stick shows it (no
//! network pill, `bc1` addresses, `xpub`s): the session stays on
//! Mainnet and the test wallets the tour makes in process are made on
//! it (`faraday_core::testkit::use_network`); `TESTKIT_DIR` must then be
//! what `faraday-testkit --network mainnet` wrote. Without it, both are
//! on testnet. The `modes` tour's Testnet screens stay on testnet.
//!
//! `TESTKIT_DIR` is what `faraday-testkit` wrote; its files stand in
//! for a stick called TESTSTICK. Each step of the tour is one numbered
//! PNG in `OUT_DIR`. With `spend`, only the Spend tab's tour runs; with
//! `devices`, the stick's device check and its reminders; with
//! `themes`, three screens in each theme; with `keys`, Wallets with keys
//! in no wallet; with `visit`, Stick visit with a backup's 20 files in the
//! Outbox, each list scrolled to its end; with `copy`, the backup's seeds
//! step checking the copy, without a camera and with one; with `scan`,
//! Home's Scan pressed with a stick in, then the camera once it is pulled;
//! with `public`, the public files offered as a code and a picture (a
//! signed message, Silent payments, Create's keys and Public files card,
//! the QR sheet, the backup's public files item, a GPG key in a vault), and the
//! pictures put in the Outbox written beside the screens as `outbox-*`;
//! with `seedfile`, the backup's checklist offering the seed into a
//! vault or as a file, with no vault open and with the test vault, the
//! secret sheet for the seed, and its SeedQR pictures, Standard and
//! Compact, written beside the screens as `outbox-*`; with `vaultway`,
//! Tools' GPG key with no vault, Create a vault made for it, the tile
//! with the test vault locked, Unlock for it and the GPG keys it comes
//! back to; with `kept`, the backup's copy item and its map
//! for Savings (2-of-3, one seed here, put out as a file, no vault) and
//! for Spending (one key, in the open test vault, its copy checked); with
//! `again`, a wallet loaded again: Savings with no seed here, opening its
//! backup on the presets and its public files, then a one-key wallet
//! restored from test key 2, its checklist saving it into the test vault,
//! and the wallet chip's list of the two; with `plan`, the backup's plan
//! by place (`docs/BACKUP-FLOW.md` §15): the presets, the places, each
//! place's card, a vault's sheet, where the description goes and The
//! plan for a one-key wallet; for Savings (2-of-3) the description whole
//! and in shares, the page What is a share? opens, the checklist part
//! done, the blank template written beside the screens as `outbox-*`; a
//! 3-of-5's shares on the description card and the checklist; and a
//! vault's sheet with two seeds; with `board`, the backup board
//! (`docs/BACKUP-FLOW.md` §17) for Savings with its three keys here: on
//! Paper and vault, a seed selected with the zones that take it edged,
//! a placed seed's forms open, a share placed with the keys left off,
//! and the software place; with `inbox`, Stick visit offering the Inbox's
//! files under "From the Inbox" (a wallet, a vault, a wallet's QR
//! picture, seed words and text), Select all, and the secret sheet for
//! the text and for the words; with `transfer`, the desktop app's
//! Transfer over a Downloads folder made in `OUT_DIR/downloads` (a PSBT,
//! a descriptor, a 100 KB file and one over 256 KiB): the list, a PSBT
//! sent, the larger file refused, and Receive saving a descriptor and a
//! seed's words sent from a device; with `upgrade`, Upgrade a Faraday
//! stick from Settings, the lock it asks for with a key loaded, and each
//! step with the boot copier's answers given by hand: the source read, an
//! older stick, a newer one beside it, one too small, one pulled during
//! the write, and one written; with `glance`, the Wallets card's chart
//! at a glance (`docs/history/NEW-WALLET.md` §6): a one-key wallet with no plan,
//! then with Paper and vault, and a 2-of-3 with two keys here, the second
//! with a passphrase, and a cosigner, with Paper and vault and the
//! passphrase in the vault too; on a small panel the card's At a glance
//! row and the chart's own page; then an open vault holding the one-key
//! wallet with its plan, its chart from the backup up (§7.2); with
//! `chartedit`, the chart's changes (§9.5–§9.7): a move and a copy
//! destroyed, each asked with the check before and after, the chart once
//! moved, a line marked lost and a place marked exposed with what
//! follows, a vault copied to another stick, and a cosigner's holder
//! named; with `restore`, Restore a wallet's cards (§12): Kind with the description
//! shortcut, Quorum, Seeds with a seed matched and one refused, Check and
//! Done; with `passseeds`, a vault keeping a 2-of-3's third seed without
//! the passphrase the wallet uses it with (F62): its Load sheet, Seeds
//! with no passphrase stored from the wallet's chart, a passphrase that
//! gives another key, and the right one; with `flow`, the backup flow (`docs/BACKUP-FLOW.md`) for a GPG key
//! kept in the test vault and a new vault at three places: Where, the
//! first stick with the vault choice, the place confirmed, two empty sticks
//! read alike and the hold, Vaults, Make, the Lock card, each stick
//! written and Done; with `vaultshow`, the test vault's key with its
//! SeedQR held (`docs/BACKUP-FLOW.md` §17.7), its Save to a stick, and a
//! wallet's Save to a stick; with `guide`, the pictures in
//! `docs/GUIDE.md` §2 and §3: a fresh stick session's Home, the power
//! sheet, and New key from Wallets, from the dice transcript; with
//! `tour`, the user guide's and the website tour's pictures (run it with
//! `--network mainnet`): Home with nothing loaded, the vault read from
//! the boot stick at power-on, New key from dice, a SeedQR scanned and a
//! wallet's missing key scanned, a 2-of-3 made and its backup board
//! edited by a finger's drag, its menus and sheets, a backup's checklist
//! and sticks written, a spend signed with its animated code, a stick
//! visit, Files and a vault, the Spend tab's map and a restore.
//!
//! `@DPI` defaults to 160 (a desktop monitor); a real panel must give
//! its own, since the core picks `small`/`medium`/`wide` from physical
//! width, not pixels. The Pi's Waveshare panel is `480x640@286`; the
//! smallest panel the design supports is `240x320@143`.

mod tour;
mod tours;

use crate::tour::{Tour, display};
use faraday_core::{Action, Faraday, Medium};
use faraday_crypto::work::Compute;
use faraday_shell_api::App;
use std::path::Path;
use std::process::ExitCode;

/// Costs what the tour it runs costs.
fn run(
    c: &Compute,
    size: (u16, u16),
    dpi: u16,
    kit: &Path,
    out: &Path,
    only: Option<&str>,
    medium: Medium,
) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut app = Faraday::new();
    app.medium = medium;
    app.event(display(size, dpi));
    // The session goes to the test wallets' network (testnet unless
    // `--network` chose another), since a session on one network takes
    // none of another's wallets once a key is loaded.
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::net())
        .map_err(|e| e.text())?;
    // `FARADAY_SNAP_THEME=<id>` takes the tour in that theme ("light",
    // "dark" …) instead of the first start's.
    if let Ok(id) = std::env::var("FARADAY_SNAP_THEME")
        && let Some(theme) = faraday_core::ui::Theme::from_id(&id)
    {
        app.press(Action::Theme(theme));
    }
    let mut t = Tour {
        app,
        out: out.to_path_buf(),
        kit: kit.to_path_buf(),
        n: 0,
        now: 0,
        sticks: Vec::new(),
        answers: 0,
        dpi,
        host: None,
        boxes: None,
        go_on: true,
    };
    match only {
        Some("spend") => return tours::spend::spend_tour(&mut t),
        Some("devices") => return tours::devices::devices_tour(&mut t),
        Some("themes") => return tours::themes::themes_tour(&mut t),
        Some("compact") => return tours::compact::compact_tour(&mut t),
        Some("seeds") => return tours::seeds::seeds_tour(&mut t),
        Some("keys") => return tours::keys::keys_tour(&mut t),
        Some("visit") => return tours::visit::visit_tour(c, &mut t),
        Some("copy") => return tours::copy::copy_tour(c, &mut t),
        Some("scan") => return tours::devices::scan_tour(&mut t),
        Some("public") => return tours::public::public_tour(c, &mut t),
        Some("seedfile") => return tours::seedfile::seedfile_tour(c, &mut t),
        Some("vaultway") => return tours::vaultway::vaultway_tour(&mut t),
        Some("kept") => return tours::kept::kept_tour(c, &mut t),
        Some("again") => return tours::again::again_tour(&mut t),
        Some("plan") => return tours::plan::plan_tour(c, &mut t),
        Some("board") => return tours::board::board_tour(c, &mut t),
        Some("menu") => return tours::menu::menu_tour(&mut t),
        Some("inbox") => return tours::inbox::inbox_tour(&mut t),
        Some("transfer") => return tours::transfer::transfer_tour(&mut t),
        Some("upgrade") => return tours::upgrade::upgrade_tour(&mut t),
        Some("glance") => return tours::glance::glance_tour(c, &mut t),
        Some("chartedit") => return tours::chart_edit::chart_edit_tour(c, &mut t),
        Some("fromvault") => return tours::from_vault::from_vault_tour(c, &mut t),
        Some("first") => return tours::first::first_tour(c, &mut t),
        Some("roller") => return tours::roller::roller_tour(&mut t),
        Some("dice5") => return tours::dice_five::dice_five_tour(&mut t),
        Some("restore") => return tours::restore::restore_tour(&mut t),
        Some("osk") => return tours::devices::osk_tour(&mut t),
        Some("flow") => return tours::flow::flow_tour(&mut t),
        Some("bootplace") => return tours::boot_place::boot_place_tour(&mut t),
        Some("passseeds") => return tours::pass_seeds::pass_seeds_tour(c, &mut t),
        Some("power") => return tours::devices::power_tour(&mut t),
        Some("pills") => return tours::desktop::pills_tour(&mut t),
        Some("walletflow") => return tours::wallet_flow::wallet_flow_tour(c, &mut t),
        Some("demo") => return tours::desktop::demo_tour(&mut t),
        Some("modes") => return tours::desktop::modes_tour(&mut t),
        Some("vaultshow") => return tours::vault_show::vault_show_tour(&mut t),
        Some("addkey") => return tours::add_key::add_key_tour(&mut t),
        Some("codex32correct") => {
            return tours::codex32_correct::codex32_correct_tour(c, &mut t);
        }
        Some("createfrom") => return tours::create_from::create_from_tour(c, &mut t),
        Some("inprogress") => return tours::in_progress::in_progress_tour(&mut t),
        Some("cancel") => return tours::cancel::cancel_tour(&mut t),
        Some("std") => return tours::standard::standard_tour(&mut t),
        Some("parts") => return tours::partials::partials_tour(&mut t),
        Some("guide") => return tours::guide::guide_tour(c, &mut t),
        Some("tour") => return tours::site_tour::site_tour(c, &mut t),
        Some("codex32") => return tours::codex32::codex32_tour(&mut t),
        Some("codex32form") => return tours::codex32_form::codex32_form_tour(&mut t),
        Some("createshares") => return tours::create_shares::create_shares_tour(&mut t),
        _ => {}
    }
    tours::default::default_tour(c, &mut t, medium)
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().collect();
    let medium = match args.iter().position(|a| a == "--sd-card") {
        Some(i) => {
            args.remove(i);
            Medium::SdCard
        }
        None => Medium::Stick,
    };
    if let Some(i) = args.iter().position(|a| a == "--network") {
        let Some(network) = args
            .get(i + 1)
            .and_then(|n| faraday_bip::keys::Network::from_name(n))
        else {
            eprintln!("--network takes mainnet, testnet, signet or regtest");
            return ExitCode::from(2);
        };
        args.drain(i..=i + 1);
        if faraday_core::testkit::use_network(network).is_err() {
            return ExitCode::FAILURE;
        }
    }
    if !(4..=5).contains(&args.len()) {
        eprintln!(
            "usage: faraday-snapshot [--sd-card] [--network NAME] WxH[@DPI] TESTKIT_DIR OUT_DIR [spend|themes|compact|seeds|keys|visit|copy|scan|public|seedfile|vaultway|kept|again|plan|inbox|transfer|upgrade|glance|chartedit|fromvault|first|restore|osk|flow|walletflow|demo|modes|vaultshow|addkey|cancel|guide|tour|codex32]"
        );
        return ExitCode::from(2);
    }
    let (size_arg, dpi_arg) = match args[1].split_once('@') {
        Some((s, d)) => (s, Some(d)),
        None => (args[1].as_str(), None),
    };
    let Some((w, h)) = size_arg
        .split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
    else {
        eprintln!("size must be WxH[@DPI]");
        return ExitCode::from(2);
    };
    // 160 is a desktop monitor's rough density; a real panel (the Pi's
    // 480x640 at 286 dpi, or the 240x320 minimum at 143) must say its
    // own, because the core picks its size class from physical width,
    // not from pixels.
    let dpi: u16 = match dpi_arg.map(str::parse) {
        Some(Ok(d)) => d,
        Some(Err(_)) => {
            eprintln!("dpi must be a number");
            return ExitCode::from(2);
        }
        None => 160,
    };
    // The tool's token, for the costly work it does outside the app's
    // events once that work takes one (docs/COMPUTE.md §3). A tool makes
    // no drawing claim, so it counts nothing.
    #[expect(
        clippy::disallowed_methods,
        reason = "a mint: the snapshot tool (docs/COMPUTE.md §3)"
    )]
    let compute = faraday_crypto::work::Compute::new(|_| {});
    let only = args.get(4).map(String::as_str);
    if only.is_some_and(|m| {
        ![
            "spend",
            "devices",
            "themes",
            "compact",
            "seeds",
            "keys",
            "visit",
            "copy",
            "scan",
            "public",
            "seedfile",
            "vaultway",
            "kept",
            "again",
            "plan",
            "board",
            "menu",
            "inbox",
            "transfer",
            "upgrade",
            "glance",
            "chartedit",
            "fromvault",
            "first",
            "restore",
            "osk",
            "flow",
            "bootplace",
            "passseeds",
            "power",
            "pills",
            "walletflow",
            "demo",
            "modes",
            "vaultshow",
            "addkey",
            "codex32correct",
            "createfrom",
            "inprogress",
            "cancel",
            "std",
            "parts",
            "guide",
            "roller",
            "dice5",
            "tour",
            "codex32",
            "codex32form",
            "createshares",
        ]
        .contains(&m)
    }) {
        eprintln!(
            "the tour is spend, devices, themes, compact, seeds, keys, visit, copy, scan, public, seedfile, vaultway, kept, again, plan, board, menu, inbox, transfer, upgrade, glance, chartedit, fromvault, first, restore, osk, flow, bootplace, passseeds, power, pills, walletflow, demo, modes, vaultshow, addkey, codex32correct, createfrom, inprogress, cancel, std, parts, guide, roller, dice5, codex32, codex32form, createshares or tour"
        );
        return ExitCode::from(2);
    }
    match run(
        &compute,
        (w, h),
        dpi,
        Path::new(&args[2]),
        Path::new(&args[3]),
        only,
        medium,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("faraday-snapshot: {e}");
            ExitCode::FAILURE
        }
    }
}
