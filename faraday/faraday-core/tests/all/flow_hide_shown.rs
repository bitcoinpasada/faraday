//! A flow on the container hides every secret shown on a step when the step
//! is left, by any way out, so the step reopens with it hidden
//! (`docs/FLOW-CONTAINER.md` §3, F104): the eye is off and no secret text
//! is drawn.
//!
//! Flows that show no secret on any step, so no test: Back up's plan and
//! board, Restore (its passphrase is asked on Add a key's form, outside
//! the flow), Check a signer, Check a timestamp proof, Sign or verify a
//! message, Silent payments. Make a new key is not a contained flow yet.

use crate::common::type_text;
use faraday_core::testkit::{self, Plain};
use faraday_core::vault::{VaultAction as V, vstep};
use faraday_core::{Action, ChoicesPress, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

const SECRET: &str = "hidepassword";

fn window(app: &mut Faraday, width: u16, height: u16) {
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// Frames and ticks a step change's live fade takes to finish.
fn at_rest(app: &mut Faraday) {
    let mut now = app.now_ms();
    for _ in 0..30 {
        now += 30;
        app.event(Event::Tick { now_ms: now });
        let _ = app.frame();
    }
}

/// Whether the screen draws `want`, whatever the spaces between groups.
fn draws(app: &mut Faraday, want: &str) -> bool {
    let _ = app.frame();
    let all: String = app
        .drawn_text_boxes()
        .plain()
        .into_iter()
        .map(|(t, _)| t)
        .collect::<String>()
        .split_whitespace()
        .collect();
    all.contains(want)
}

fn on_make_a_vault(width: u16, height: u16) -> Faraday {
    let mut app = Faraday::new();
    window(&mut app, width, height);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    testkit::settle(&mut app);
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, SECRET);
    app.press(Action::Vault(V::CShow));
    assert!(draws(&mut app, SECRET), "shown");
    app
}

/// Make a new vault: Next, the pane's Opens on row and Back.
#[test]
fn make_a_new_vault_hides_the_passphrase_by_every_way_out() {
    for way in ["back then forward", "pane row", "sheet row"] {
        let narrow = way == "sheet row";
        let mut app = if narrow {
            on_make_a_vault(480, 640)
        } else {
            on_make_a_vault(1366, 768)
        };
        match way {
            "back then forward" => {
                let _ = app.frame();
                app.event(Event::Back);
                at_rest(&mut app);
                app.event(Event::Forward);
            }
            "pane row" => {
                app.press(Action::Vault(V::CStep(vstep::PRESET)));
                at_rest(&mut app);
                app.press(Action::Vault(V::CStep(vstep::PHRASES)));
            }
            _ => {
                app.press(Action::Choices(ChoicesPress::Open));
                let _ = app.frame();
                app.press(Action::Choices(ChoicesPress::Row(vstep::PRESET)));
                at_rest(&mut app);
                app.press(Action::Choices(ChoicesPress::Open));
                let _ = app.frame();
                app.press(Action::Choices(ChoicesPress::Row(vstep::PHRASES)));
            }
        }
        at_rest(&mut app);
        assert_eq!(
            app.vaults().create.as_ref().and_then(|c| c.open),
            Some(vstep::PHRASES),
            "{way}: back on the passphrases"
        );
        assert!(
            !app.vaults().create.as_ref().unwrap().shown,
            "{way}: eye off"
        );
        assert!(!draws(&mut app, SECRET), "{way}: no secret text drawn");
    }
}

const ABOUT: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// My seed makes my wallet: the passphrase's eye, left by Back and opened
/// again, and left by a row of Your choices.
#[test]
fn my_seed_makes_my_wallet_hides_the_passphrase_by_every_way_out() {
    use faraday_core::seed_check::SeedAction as S;
    for way in ["back", "sheet row"] {
        let mut app = Faraday::new();
        window(&mut app, 480, 640);
        testkit::move_to(&mut app.session, faraday_bip::keys::Network::Mainnet).unwrap();
        app.press(Action::Nav(Screen::Catalog));
        let tile = faraday_core::catalog::TILES
            .iter()
            .position(|t| t.name == "My seed makes my wallet")
            .unwrap();
        app.press(Action::Catalog(tile as u8));
        app.press(Action::SeedCheck(S::Type));
        testkit::type_words(&mut app, ABOUT);
        app.press(Action::SeedCheck(S::Words));
        app.press(Action::SeedCheck(S::AddPass));
        app.press(Action::SeedCheck(S::PassField(0)));
        type_text(&mut app, SECRET);
        app.press(Action::PassEye);
        assert!(draws(&mut app, SECRET), "{way}: shown");
        if way == "back" {
            let _ = app.frame();
            app.event(Event::Back);
            let _ = app.frame();
        } else {
            app.press(Action::Choices(ChoicesPress::Open));
            let _ = app.frame();
            app.press(Action::Choices(ChoicesPress::Row(0)));
            at_rest(&mut app);
            app.event(Event::Forward);
        }
        at_rest(&mut app);
        assert!(!draws(&mut app, SECRET), "{way}: no secret text drawn");
    }
}

/// BIP-85: the child shown on the result is hidden once the result is left
/// by Back or a row of Your choices, and stays hidden on return.
#[test]
fn bip85_hides_the_child_by_every_way_out() {
    for way in ["back", "sheet row"] {
        let mut app = Faraday::new();
        window(&mut app, 480, 640);
        testkit::move_to(&mut app.session, testkit::NET).unwrap();
        app.press(Action::Entry(None));
        type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
        app.press(Action::EntryAdd);
        testkit::keep_published(&mut app);
        app.press(Action::Bip85);
        app.press(Action::PApp(4));
        app.press(Action::PNext);
        app.press(Action::PNext);
        app.press(Action::PShow);
        let _ = app.frame();
        assert!(app.bip85().is_some_and(|b| b.shown), "{way}: shown");
        if way == "back" {
            app.event(Event::Back);
            at_rest(&mut app);
            app.event(Event::Forward);
        } else {
            app.press(Action::Choices(ChoicesPress::Open));
            let _ = app.frame();
            app.press(Action::Choices(ChoicesPress::Row(0)));
            at_rest(&mut app);
            app.press(Action::PStep(3));
        }
        at_rest(&mut app);
        assert!(!app.bip85().is_some_and(|b| b.shown), "{way}: eye off");
    }
}

/// Read a Bitcoin Core wallet: the passphrase shown with the eye is hidden
/// once its step is left by Back.
#[test]
fn read_a_core_wallet_hides_the_passphrase_by_back() {
    use faraday_core::core_wallet::tool::CoreWalletAction as W;
    let file = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/vectors/core-wallet/encrypted.dat"
    ))
    .unwrap();
    let mut app = testkit::started();
    app.session = faraday_core::wallet::Session::on(faraday_bip::keys::Network::Testnet);
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![("wallet.dat".to_string(), file)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Read a Bitcoin Core wallet")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    app.press(Action::CoreWallet(W::Focus));
    type_text(&mut app, SECRET);
    app.press(Action::CoreWallet(W::Eye));
    assert!(draws(&mut app, SECRET), "shown");
    let _ = app.frame();
    app.event(Event::Back);
    at_rest(&mut app);
    app.event(Event::Forward);
    at_rest(&mut app);
    assert!(!draws(&mut app, SECRET), "no secret text drawn");
}

/// Convert a seed: the value shown with the eye is hidden once its step is
/// left by Back.
#[test]
fn convert_a_seed_hides_the_value_by_back() {
    use faraday_core::convert::{self, ROWS};
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABOUT, "Test", None).unwrap();
    testkit::keep_published(&mut app);
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Convert a seed")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    let row = ROWS.iter().position(|r| r.name == "xprv").unwrap() as u8;
    app.press(Action::Convert(faraday_core::convert::ConvertAction::To(
        row,
    )));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(&mut app);
    app.press(Action::Convert(faraday_core::convert::ConvertAction::Show));
    assert!(convert::shown(&app).is_some(), "shown");
    let _ = app.frame();
    app.event(Event::Back);
    at_rest(&mut app);
    app.event(Event::Forward);
    at_rest(&mut app);
    assert!(convert::shown(&app).is_none(), "hidden again");
}

/// Leaves the step the app is on, by Back (and Forward again) or by the
/// first row of Your choices, and lets the fade finish.
fn leave(app: &mut Faraday, way: &str) {
    leave_by(app, way, 0);
}

/// [`leave`], the row of Your choices being `row`.
fn leave_by(app: &mut Faraday, way: &str, row: u8) {
    let _ = app.frame();
    if way == "back" {
        app.event(Event::Back);
        at_rest(app);
        app.event(Event::Forward);
    } else {
        app.press(Action::Choices(ChoicesPress::Open));
        let _ = app.frame();
        app.press(Action::Choices(ChoicesPress::Row(row)));
    }
    at_rest(app);
}

/// A one-key wallet from test key 2 typed in; its index.
fn one_key_wallet(app: &mut Faraday) -> usize {
    use faraday_core::seeds::SeedsAction as S;
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().fingerprint;
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(S::Make));
    app.session.wallets.len() - 1
}

fn on_testnet(width: u16, height: u16) -> Faraday {
    let mut app = Faraday::new();
    window(&mut app, width, height);
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app
}

/// Back up, the quick paper backup: the words, and the SeedQR beside them,
/// shown on the first screen are hidden once it is left.
#[test]
fn the_quick_paper_backup_hides_the_words_and_the_seedqr() {
    use faraday_core::wallet_backup::quick::Quick;
    for (way, seedqr) in [("back", false), ("sheet row", false), ("back", true)] {
        let mut app = on_testnet(1366, 768);
        let w = one_key_wallet(&mut app);
        app.press(Action::BQuick(Quick::Open(w)));
        assert!(app.backup().is_some_and(|b| b.quick), "not the quick path");
        app.press(Action::BQuick(Quick::Show));
        if seedqr {
            app.press(Action::BQuick(Quick::SeedQr));
        }
        assert!(app.backup().is_some_and(|b| b.reveal), "{way}: shown");
        leave_by(&mut app, way, 201);
        assert!(
            !app.backup().is_some_and(|b| b.reveal),
            "{way} (SeedQR {seedqr}): the words are hidden"
        );
    }
}

/// Back up's checklist: the seed's words shown on the copy item are hidden
/// once the step is left.
#[test]
fn the_backup_checklist_hides_the_words() {
    use faraday_core::plan::{Item, Preset};
    for way in ["back", "sheet row"] {
        let mut app = on_testnet(1366, 768);
        let w = one_key_wallet(&mut app);
        app.press(Action::Backup(w));
        testkit::backup_as(&mut app, Preset::PaperVault);
        app.press(Action::BChecklist);
        app.press(Action::BStep(Item::Copy(0)));
        app.press(Action::BReveal);
        assert!(app.backup().is_some_and(|b| b.reveal), "{way}: shown");
        leave(&mut app, way);
        assert!(
            !app.backup().is_some_and(|b| b.reveal),
            "{way}: the words are hidden"
        );
    }
}

/// Back up's vault flow: the passphrase of a vault being unlocked, shown
/// with the eye, is hidden once the step is left.
#[test]
fn the_backup_flow_hides_a_typed_vault_passphrase() {
    use crate::common::{Disk, Shell, keep_time, process, start, vault};
    use faraday_core::backup_flow::Kind;
    use faraday_core::backup_flow::steps::FlowAction as F;
    for way in ["back", "sheet row"] {
        let mut shell = Shell::light();
        let mut d = Disk::empty("S1", "WORKSTICK");
        d.files = vec![("main.kdbx".into(), vault("Main", SECRET, 0x44))];
        shell.add(d);
        shell.add(Disk::empty("S2", "SPARE"));
        shell.disks[1].serial = Some(7);
        let mut app = process(Vec::new(), Vec::new(), Vec::new());
        keep_time(&mut app);
        start(&mut app, Kind::Gpg);
        shell.flow(&mut app, F::VaultExisting(0));
        shell.flow(&mut app, F::Continue);
        shell.plug(&mut app, "S1");
        shell.flow(&mut app, F::PickFile(0, 0));
        shell.flow(&mut app, F::Tap);
        shell.pull(&mut app, "S1");
        shell.plug(&mut app, "S2");
        shell.flow(&mut app, F::Tap);
        shell.pull(&mut app, "S2");
        shell.flow(&mut app, F::UnlockVault(0));
        at_rest(&mut app);
        app.press(Action::Vault(V::FocusPassphrase));
        type_text(&mut app, SECRET);
        app.press(Action::Vault(V::ShowTyped));
        assert!(app.vaults().typed_shown, "{way}: shown");
        leave(&mut app, way);
        assert!(!app.vaults().typed_shown, "{way}: the passphrase is hidden");
    }
}

/// Spend from a backup: the vault passphrase typed for the unlock and
/// shown is hidden once the step is left.
#[test]
fn spend_from_a_backup_hides_the_typed_vault_passphrase() {
    use faraday_core::spend_backup::{SpendBackupAction as F, page};
    for way in ["back", "sheet row"] {
        let mut app = on_testnet(1366, 768);
        app.storage(faraday_core::StorageEvent::Restored {
            inbox: faraday_core::files_from(vec![(
                "vault.kdbx".to_string(),
                testkit::test_vault().unwrap(),
            )]),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app.press(Action::Nav(Screen::SpendBackup));
        app.press(Action::SpendBackup(F::Next(page::MAP)));
        at_rest(&mut app);
        app.press(Action::SpendBackup(F::Pick(0)));
        type_text(&mut app, testkit::VAULT_PASSPHRASE);
        app.press(Action::Vault(V::ShowTyped));
        assert!(app.vaults().typed_shown, "{way}: shown");
        leave(&mut app, way);
        assert!(!app.vaults().typed_shown, "{way}: the passphrase is hidden");
    }
}

/// Check a multisig setup: the passphrase typed for the seed checked and
/// shown is hidden once the step is left.
#[test]
fn check_a_multisig_setup_hides_the_passphrase() {
    use faraday_core::multisig_check::MsAction as M;
    use faraday_core::seed_check::SeedAction as S;
    for way in ["back", "sheet row"] {
        let descriptor = testkit::kits()
            .into_iter()
            .find(|k| k.id == "savings")
            .unwrap()
            .descriptor;
        let mut app = testkit::started();
        app.press(Action::MultisigCheck(M::Open));
        app.press(Action::MultisigCheck(M::Scan));
        app.event(Event::Scanned {
            bytes: descriptor.into_bytes(),
        });
        at_rest(&mut app);
        app.press(Action::MultisigCheck(M::CheckSeed));
        at_rest(&mut app);
        app.press(Action::SeedCheck(S::Type));
        testkit::type_words(&mut app, ABOUT);
        app.press(Action::SeedCheck(S::Words));
        app.press(Action::SeedCheck(S::AddPass));
        app.press(Action::SeedCheck(S::PassField(0)));
        type_text(&mut app, SECRET);
        app.press(Action::PassEye);
        assert!(draws(&mut app, SECRET), "{way}: shown");
        leave(&mut app, way);
        assert!(!draws(&mut app, SECRET), "{way}: no secret text drawn");
    }
}

/// Create wallet: the passphrase typed for a key in a slot and shown with
/// its eye is hidden once the step is left.
#[test]
fn create_wallet_hides_the_slot_passphrase() {
    use faraday_core::cstep;
    for way in ["back", "sheet row"] {
        let mut app = testkit::started();
        testkit::move_to(&mut app.session, testkit::NET).unwrap();
        let c = testkit::compute();
        app.session.keys.add_words(&c, ABOUT, "Test", None).unwrap();
        let fp = app.session.keys.last().unwrap().fingerprint.0;
        app.press(Action::CreateWallet);
        crate::common::ckind(&mut app, 0);
        app.press(Action::CNext(cstep::KIND));
        app.press(Action::CSlotHere(0, fp));
        at_rest(&mut app);
        app.press(Action::CPassOpen(0));
        type_text(&mut app, SECRET);
        app.press(Action::CPassField(1));
        type_text(&mut app, SECRET);
        app.press(Action::PassEye);
        assert!(draws(&mut app, SECRET), "{way}: shown");
        leave(&mut app, way);
        assert!(!draws(&mut app, SECRET), "{way}: no secret text drawn");
    }
}

/// Krux code tool: the passphrase typed for a code and shown with its eye
/// is hidden once the step is left.
#[test]
fn the_krux_code_tool_hides_the_passphrase() {
    use faraday_core::kef_form::KefAct;
    use faraday_core::kef_tool::{KefToolAction as K, Pick};
    for way in ["back", "sheet row"] {
        let mut app = testkit::started();
        app.press(Action::Nav(Screen::Catalog));
        let tile = faraday_core::catalog::TILES
            .iter()
            .position(|t| t.name == "Krux code")
            .unwrap();
        app.press(Action::Catalog(tile as u8));
        app.press(Action::KefTool(K::Pick(Pick::Type)));
        type_text(&mut app, "a note for later");
        app.press(Action::KefTool(K::Next));
        at_rest(&mut app);
        app.press(Action::KefTool(K::Form(KefAct::Field)));
        type_text(&mut app, SECRET);
        app.press(Action::KefTool(K::Form(KefAct::Eye)));
        assert!(draws(&mut app, SECRET), "{way}: shown");
        leave(&mut app, way);
        assert!(!draws(&mut app, SECRET), "{way}: no secret text drawn");
    }
}

/// Vanity addresses: the passphrase a find on the passphrase dial names,
/// shown, is hidden once the step is left.
#[test]
fn vanity_hides_the_passphrase_a_find_names() {
    use faraday_core::grind::Dial;
    use faraday_core::vanity::{VanityAction as V, vstep};
    use faraday_shell_api::Key;
    for way in ["back", "sheet row"] {
        let mut app = testkit::started();
        testkit::move_to(&mut app.session, testkit::NET).unwrap();
        app.press(Action::Entry(None));
        type_text(&mut app, &testkit::test_words("bacon"));
        app.press(Action::EntryAdd);
        app.press(Action::Vanity(V::Open));
        let dial = Dial::ALL
            .iter()
            .position(|d| *d == Dial::Passphrase)
            .unwrap();
        app.press(Action::Vanity(V::Dial(dial as u8)));
        app.press(Action::Vanity(V::Next(vstep::DIAL)));
        app.press(Action::Vanity(V::Next(vstep::SCRIPT)));
        app.event(Event::Key(Key::Char('q')));
        app.press(Action::Vanity(V::Next(vstep::PREFIX)));
        app.press(Action::Vanity(V::Start));
        let mut found = false;
        for t in 1..5000u64 {
            app.event(Event::Tick { now_ms: t * 100 });
            if app.vanity().is_some_and(|v| v.grind.find.is_some()) {
                found = true;
                break;
            }
        }
        assert!(found, "no find");
        at_rest(&mut app);
        app.press(Action::Vanity(V::Show));
        assert!(app.vanity().is_some_and(|v| v.shown), "{way}: shown");
        leave(&mut app, way);
        assert!(
            !app.vanity().is_some_and(|v| v.shown),
            "{way}: the passphrase is hidden"
        );
    }
}

/// Make a new key: the words shown with Show are hidden once their step is
/// left by Back, and stay hidden on return.
#[test]
fn make_a_new_key_hides_the_words_by_back() {
    use faraday_core::keygen::Start;
    let words = testkit::test_words("bacon");
    let mut app = testkit::started();
    window(&mut app, 1366, 768);
    crate::common::add_a_key_flow(&mut app);
    at_rest(&mut app);
    app.press(Action::KStart(
        Start::Type(faraday_core::forms::Form::Words).index(),
    ));
    at_rest(&mut app);
    type_text(&mut app, &words);
    app.press(Action::KNext);
    at_rest(&mut app);
    app.press(Action::KShow);
    assert!(app.keygen().is_some_and(|k| k.shown), "shown");
    let _ = app.frame();
    app.event(Event::Back);
    at_rest(&mut app);
    app.event(Event::Forward);
    at_rest(&mut app);
    assert!(!app.keygen().is_some_and(|k| k.shown), "eye off");
}
