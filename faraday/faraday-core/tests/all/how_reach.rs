//! Every action of the manifest with a screen is one press from that
//! screen's Cryptography button (two on the Pi's panel, through the ⋮),
//! its block is on the tab it opens, and the guide opens there at a page
//! with the Cryptography tab beside it (`docs/CRYPTO-LEARN.md` §9.1). The blocks and their
//! screens are `faraday-guide`'s generated ones.

use faraday_core::nav::{BarTool, MenuPress, Overflow};
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_guide::how::BLOCKS;
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const SIZES: [(&str, u16, u16, u16); 3] = [
    ("480x640", 480, 640, 160),
    ("1280x800", 1280, 800, 160),
    ("the Pi's panel", 480, 640, 286),
];

fn screen(name: &str) -> Screen {
    use Screen as S;
    match name {
        "Home" => S::Home,
        "WalletsStart" => S::WalletsStart,
        "Wallets" => S::Wallets,
        "Spend" => S::Spend,
        "Files" => S::Files,
        "Entry" => S::Entry,
        "Backup" => S::Backup,
        "Message" => S::Message,
        "CheckMessage" => S::CheckMessage,
        "Create" => S::Create,
        "Restore" => S::Restore,
        "Settings" => S::Settings,
        "SetTime" => S::SetTime,
        "Randomness" => S::Randomness,
        "Vaults" => S::Vaults,
        "Backups" => S::Backups,
        "CreateVault" => S::CreateVault,
        "KeyGen" => S::KeyGen,
        "Bip85" => S::Bip85,
        "Convert" => S::Convert,
        "Silent" => S::Silent,
        "Explore" => S::Explore,
        "Lightning" => S::Lightning,
        "Tools" => S::Tools,
        "Unlock" => S::Unlock,
        "VaultContents" => S::VaultContents,
        "Codes" => S::Codes,
        "SpendBackup" => S::SpendBackup,
        "Vanity" => S::Vanity,
        "Decode" => S::Decode,
        "NonceCheck" => S::NonceCheck,
        "SignerCheck" => S::SignerCheck,
        "Catalog" => S::Catalog,
        "Transfer" => S::Transfer,
        "BackupFlow" => S::BackupFlow,
        "KefTool" => S::KefTool,
        "QrTool" => S::QrTool,
        "OtsCheck" => S::OtsCheck,
        "CoreWallet" => S::CoreWallet,
        "Invoice" => S::Invoice,
        "Compare" => S::Compare,
        other => panic!("{other} is not a Screen"),
    }
}

fn app(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = Faraday::new();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    // The online app, which Transfer needs.
    app.online = true;
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app
}

/// The overflow the bar offers, if it has one.
fn overflow(app: &Faraday) -> Option<Overflow> {
    (0u8..32)
        .map(|bits| Overflow {
            own: bits & 16 != 0,
            how: bits & 1 != 0,
            help: bits & 2 != 0,
            keyboard: bits & 4 != 0,
            keyboard_up: false,
            eye: bits & 8 != 0,
        })
        .filter(|held| app.offers(Action::NavMenu(MenuPress::Open(Some(*held)))))
        .last()
}

/// Presses the Cryptography button, or its row in the ⋮.
fn press_how(app: &mut Faraday) -> bool {
    if app.offers(Action::How) {
        app.press(Action::How);
    } else if let Some(held) = overflow(app) {
        app.press(Action::NavMenu(MenuPress::Open(Some(held))));
        let _ = app.frame();
        app.press(Action::NavMenu(MenuPress::Tool(BarTool::How)));
    } else {
        return false;
    }
    let _ = app.frame();
    true
}

/// The screens the manifest's actions name, as variants' names.
fn screen_names() -> Vec<&'static str> {
    let mut names: Vec<&str> = BLOCKS
        .iter()
        .flat_map(|b| b.screens.iter().copied())
        .filter(|s| !s.starts_with("starter:"))
        .collect();
    names.sort_unstable();
    names.dedup();
    assert!(!names.is_empty());
    names
}

fn on_screen(name: &str, w: u16, h: u16, dpi: u16) -> Faraday {
    let mut app = app(w, h, dpi);
    if name == "NonceCheck" {
        // It opens on a signed file: Savings' spend as its cosigner
        // signed it, from Files.
        let (_, cosigner) = testkit::spend().unwrap();
        app.storage(StorageEvent::Restored {
            inbox: vec![("signed.psbt".to_string(), cosigner.to_bytes())],
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
            0,
        )));
    } else if name == "KefTool" {
        // It opens from its tile in Tools' catalog.
        app.press(Action::Nav(Screen::Catalog));
        let tile = faraday_core::catalog::TILES
            .iter()
            .position(|t| t.name == "Krux code")
            .unwrap();
        app.press(Action::Catalog(tile as u8));
    } else {
        app.press(Action::Nav(screen(name)));
    }
    app.settle();
    let _ = app.frame();
    assert_eq!(app.screen, screen(name), "the test puts the app on {name}");
    app
}

/// Whether a line of the drawn text is the start of `name`, which wraps.
fn name_drawn(app: &mut Faraday, name: &str) -> bool {
    app.drawn_texts()
        .iter()
        .any(|t| t.len() > 3 && name.starts_with(t.trim()))
}

#[test]
fn every_action_with_a_screen_is_one_press_from_its_block() {
    let mut failed = Vec::new();
    for name in screen_names() {
        for (size, w, h, dpi) in SIZES {
            let mut app = on_screen(name, w, h, dpi);
            if !press_how(&mut app) {
                failed.push(format!("{name} at {size}: no Cryptography button"));
                continue;
            }
            if !app.guide().how {
                failed.push(format!(
                    "{name} at {size}: Learn is not on the Cryptography tab"
                ));
            }
            let shown: Vec<&str> = app.how_blocks().iter().map(|b| b.id).collect();
            let mut first = true;
            for b in BLOCKS.iter().filter(|b| b.screens.contains(&name)) {
                if !shown.contains(&b.id) {
                    failed.push(format!("{name} at {size}: {} is not on the tab", b.id));
                }
                if first {
                    first = false;
                    if !name_drawn(&mut app, b.name) {
                        failed.push(format!("{name} at {size}: {} is not drawn", b.name));
                    }
                }
            }
        }
    }
    assert!(
        failed.is_empty(),
        "{} failures:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

/// The guide on every screen that offers an action with a block opens the
/// guide at the screen's page, with the Cryptography tab beside it.
#[test]
fn every_screen_with_actions_opens_the_guide_at_a_page_with_the_cryptography_tab() {
    let mut failed = Vec::new();
    for name in screen_names() {
        let mut app = on_screen(name, 1280, 800, 160);
        app.press(Action::Guide);
        let _ = app.frame();
        if app.guide().shown.is_empty() && !app.guide().contents {
            failed.push(format!("{name}: no page"));
        }
        if !app.offers(Action::GuideTabHow) {
            failed.push(format!("{name}: no Cryptography tab"));
        }
    }
    assert!(
        failed.is_empty(),
        "{} missing:\n{}",
        failed.len(),
        failed.join("\n")
    );
}
