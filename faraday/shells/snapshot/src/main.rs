//! Renders Faraday's screens to PNG along one scripted tour.
//!
//! ```text
//! faraday-snapshot [--sd-card] [--network mainnet] WxH[@DPI] TESTKIT_DIR OUT_DIR [spend|themes|compact|seeds|keys|visit|copy|scan|public|seedfile|vaultway|kept|again|plan|board|inbox|transfer|glance|chartedit|fromvault|first|restore|osk|flow|bootplace|passseeds|power|pills|components|guide|screens|tour|codex32]
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
//! `toasts`, two toasts on Home, one line and one that wraps; with
//! `themes`, five screens in each scheme (docs/THEMES.md); with `keys`, Wallets with keys
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
//! seed's words sent from a device; with `glance`, the Wallets card's chart
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
//! wallet's Save to a stick; with `guide`, the pictures of the guide's
//! walkthroughs (`docs/guide/walk/`), one module for each, from a fresh
//! stick session and the dice transcript's key; with `screens`, the
//! pictures of its screen reference (`docs/guide/screens/`). These two
//! name each picture as its file, with no number, and write
//! `<name>.mark.json` beside a picture whose next control is marked
//! (`just guide-pictures`); with
//! `tour`, the website tour's pictures (run it with `--network
//! mainnet`), with the guide's key: Make codex32 shares and a vault's
//! passphrase, each from dice as the rolls go in, Settings' Appearance
//! in two themes, a 2-of-3's backup board laid out, and a
//! transaction's review with its warnings.
//!
//! `@DPI` defaults to 160 (a desktop monitor); a real panel must give
//! its own, since the core picks `small`/`medium`/`wide` from physical
//! width, not pixels. The Pi's Waveshare panel is `480x640@286`; the
//! smallest panel the design supports is `240x320@143`.

mod targets;
mod tour;
mod tours;

use crate::tour::{Small, Tour, display};
use faraday_core::{Action, Faraday, Medium};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Orientation};
use std::path::Path;
use std::process::ExitCode;

/// The panel's own orientation where `size` at `dpi` is one of the
/// stick's 7-inch or 10-inch panels, either way round: the 7-inch
/// 1024 × 600 at 170 dpi, landscape natively, and the 10-inch 800 × 1280
/// at 150 dpi, portrait natively.
fn panel_native(size: (u16, u16), dpi: u16) -> Option<Orientation> {
    match (size, dpi) {
        ((1024, 600) | (600, 1024), 170) => Some(Orientation::Landscape),
        ((1280, 800) | (800, 1280), 150) => Some(Orientation::Portrait),
        _ => None,
    }
}

/// Offers Rotation as the stick does, on a 7-inch or 10-inch panel's
/// render.
fn offer_rotation(app: &mut Faraday, size: (u16, u16), dpi: u16) {
    // A 7-inch or 10-inch panel's render (`docs/ORIENTATION.md` §10):
    // the stick offers Rotation there, chosen the way this render
    // stands. The render is already that size: the turn the app asks
    // for is left unanswered.
    if let Some(native) = panel_native(size, dpi) {
        app.event(faraday_shell_api::Event::RotationOffered { native });
        if size.1 > size.0 {
            app.press(Action::Rotation(Orientation::Portrait));
        }
    }
}

/// The tours `faraday-snapshot` runs by name.
pub(crate) const TOURS: &[&str] = &[
    "spend",
    "devices",
    "toasts",
    "themes",
    "tooltips",
    "compact",
    "seeds",
    "keys",
    "visit",
    "copy",
    "scan",
    "public",
    "qr",
    "seedfile",
    "vaultway",
    "kept",
    "again",
    "plan",
    "board",
    "boardunsorted",
    "menu",
    "m3",
    "inbox",
    "silent",
    "keypath",
    "register",
    "nonce",
    "transfer",
    "glance",
    "chartedit",
    "fromvault",
    "first",
    "restore",
    "osk",
    "flow",
    "gpg-restore",
    "bootplace",
    "passseeds",
    "power",
    "pills",
    "components",
    "host",
    "walletflow",
    "demo",
    "modes",
    "vaultshow",
    "addkey",
    "keygen",
    "codex32correct",
    "createfrom",
    "inprogress",
    "cancel",
    "how",
    "std",
    "parts",
    "panes",
    "guide",
    "screens",
    "roller",
    "randomness",
    "dice5",
    "tour",
    "codex32",
    "codex32tour",
    "codex32form",
    "createshares",
    "codex32backup",
];

/// Costs what the tour it runs costs.
fn run(
    c: &Compute,
    size: (u16, u16),
    dpi: u16,
    (kit, out): (&Path, &Path),
    only: Option<&str>,
    medium: Medium,
    pictures: bool,
) -> Result<Vec<Small>, String> {
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut app = Faraday::new();
    app.medium = medium;
    app.event(display(size, dpi));
    offer_rotation(&mut app, size, dpi);
    // The session goes to the test wallets' network (testnet unless
    // `--network` chose another), since a session on one network takes
    // none of another's wallets once a key is loaded.
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::net())
        .map_err(|e| e.text())?;
    // `FARADAY_SNAP_THEME=<theme>[,<mode>[,<contrast>]]` takes the tour
    // in that scheme ("frost,dark,high"; a stored id alone, "nord",
    // opens as a settings file of it would) instead of the first start's.
    if let Ok(words) = std::env::var("FARADAY_SNAP_THEME")
        && let Some(theme) = faraday_core::ui::Theme::from_words(&words)
    {
        app.press(Action::Theme(theme));
    }
    // `FARADAY_SNAP_TEXT=large|larger` takes the tour at that text size
    // (`docs/TEXT-SIZE.md` §7), and each frame with text cut off at a
    // side or drawn over other text is listed in `cut.txt` beside the
    // frames.
    let text = std::env::var("FARADAY_SNAP_TEXT").ok();
    if let Some(words) = &text {
        let size = faraday_core::ui::TextSize::from_id(words)
            .ok_or_else(|| format!("FARADAY_SNAP_TEXT={words}: standard, large or larger"))?;
        app.press(Action::TextSize(size));
        let cut = out.join("cut.txt");
        std::fs::write(&cut, "").map_err(|e| format!("{}: {e}", cut.display()))?;
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
        named: matches!(only, Some("guide" | "screens")),
        pictures,
        targets: Some(Vec::new()),
    };
    match only.and_then(|name| named_tour(c, &mut t, name)) {
        Some(done) => done,
        None => tours::default::default_tour(c, &mut t, medium),
    }?;
    Ok(t.targets.take().unwrap_or_default())
}

/// Costs what the tour it runs costs. The tour named `name`, run;
/// `None` for a name no tour has, which runs the default tour.
fn named_tour(c: &Compute, t: &mut Tour, name: &str) -> Option<Result<(), String>> {
    Some(match name {
        "spend" => tours::spend::spend_tour(t),
        "devices" => tours::devices::devices_tour(t),
        "toasts" => tours::devices::toasts_tour(t),
        "themes" => tours::themes::themes_tour(t),
        "tooltips" => tours::tooltips::tooltips_tour(t),
        "compact" => tours::compact::compact_tour(t),
        "seeds" => tours::seeds::seeds_tour(t),
        "keys" => tours::keys::keys_tour(t),
        "visit" => tours::visit::visit_tour(c, t),
        "copy" => tours::copy::copy_tour(c, t),
        "scan" => tours::devices::scan_tour(t),
        "public" => tours::public::public_tour(c, t),
        "qr" => tours::qr::qr_tour(c, t),
        "seedfile" => tours::seedfile::seedfile_tour(c, t),
        "vaultway" => tours::vaultway::vaultway_tour(t),
        "kept" => tours::kept::kept_tour(c, t),
        "again" => tours::again::again_tour(t),
        "plan" => tours::plan::plan_tour(c, t),
        "board" => tours::board::board_tour(c, t),
        "boardunsorted" => tours::board::board_unsorted_tour(c, t),
        "menu" => tours::menu::menu_tour(t),
        "m3" => tours::m3::m3_tour(t),
        "inbox" => tours::inbox::inbox_tour(t),
        "silent" => tours::silent::silent_tour(c, t),
        "keypath" => tours::key_path::key_path_tour(t),
        "register" => tours::register::register_tour(t),
        "nonce" => tours::nonce::nonce_tour(t),
        "transfer" => tours::transfer::transfer_tour(t),
        "glance" => tours::glance::glance_tour(c, t),
        "chartedit" => tours::chart_edit::chart_edit_tour(c, t),
        "fromvault" => tours::from_vault::from_vault_tour(c, t),
        "first" => tours::first::first_tour(c, t),
        "roller" => tours::roller::roller_tour(t),
        "randomness" => tours::randomness::randomness_tour(t),
        "dice5" => tours::dice_five::dice_five_tour(t),
        "restore" => tours::restore::restore_tour(t),
        "osk" => tours::devices::osk_tour(t),
        "flow" => tours::flow::flow_tour(t),
        "gpg-restore" => tours::gpg_restore::gpg_restore_tour(t),
        "bootplace" => tours::boot_place::boot_place_tour(t),
        "passseeds" => tours::pass_seeds::pass_seeds_tour(c, t),
        "power" => tours::devices::power_tour(t),
        "pills" => tours::desktop::pills_tour(t),
        "components" => tours::components::components_tour(t),
        "host" => tours::host::host_tour(t),
        "walletflow" => tours::wallet_flow::wallet_flow_tour(c, t),
        "demo" => tours::desktop::demo_tour(t),
        "modes" => tours::desktop::modes_tour(t),
        "vaultshow" => tours::vault_show::vault_show_tour(t),
        "addkey" => tours::add_key::add_key_tour(t),
        "keygen" => tours::keygen::keygen_tour(t),
        "codex32correct" => tours::codex32_correct::codex32_correct_tour(c, t),
        "createfrom" => tours::create_from::create_from_tour(c, t),
        "inprogress" => tours::in_progress::in_progress_tour(t),
        "cancel" => tours::cancel::cancel_tour(t),
        "how" => tours::how::how_tour(t),
        "std" => tours::standard::standard_tour(t),
        "parts" => tours::partials::partials_tour(t),
        "panes" => tours::panes::panes_tour(t),
        "guide" => tours::guide::guide_tour(c, t),
        "screens" => tours::screens::screens_tour(c, t),
        "tour" => tours::site_tour::site_tour(c, t),
        "codex32" => tours::codex32::codex32_tour(t),
        "codex32tour" => tours::codex32::codex32_shares_tour(t),
        "codex32form" => tours::codex32_form::codex32_form_tour(t),
        "createshares" => tours::create_shares::create_shares_tour(t),
        "codex32backup" => tours::codex32_backup::codex32_backup_tour(t),
        _ => return None,
    })
}

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
            "usage: faraday-snapshot [--sd-card] [--network NAME] WxH[@DPI] TESTKIT_DIR OUT_DIR [spend|themes|compact|seeds|keys|visit|copy|scan|public|seedfile|vaultway|kept|again|plan|inbox|transfer|glance|chartedit|fromvault|first|restore|osk|flow|walletflow|demo|modes|vaultshow|addkey|cancel|how|guide|screens|tour|codex32]"
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
    if only.is_some_and(|m| !TOURS.contains(&m)) {
        eprintln!(
            "the tour is spend, devices, toasts, themes, tooltips, compact, seeds, keys, visit, copy, scan, public, seedfile, vaultway, kept, again, plan, board, menu, m3, inbox, transfer, glance, chartedit, fromvault, first, restore, osk, flow, gpg-restore, bootplace, passseeds, power, pills, components, walletflow, demo, modes, vaultshow, addkey, keygen, codex32correct, createfrom, inprogress, cancel, how, std, parts, panes, guide, roller, randomness, dice5, codex32, codex32tour, codex32form, createshares, codex32backup, screens or tour"
        );
        return ExitCode::from(2);
    }
    match run(
        &compute,
        (w, h),
        dpi,
        (Path::new(&args[2]), Path::new(&args[3])),
        only,
        medium,
        true,
    ) {
        Ok(small) => match targets::write(Path::new(&args[3]), only, &small) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("faraday-snapshot: {e}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("faraday-snapshot: {e}");
            ExitCode::FAILURE
        }
    }
}
