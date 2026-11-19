//! One tour over every place that can hold a typed or scanned secret
//! (`docs/SECRET-LEAKS.md` §4). A row plants a test secret there through
//! the app's own input and then:
//!
//! 1. the app is not counted clean while it is held;
//! 2. Lock, the save failed (the old app is moved to a fresh one in the
//!    same process, `Faraday::carry_unkept_lock`): no live block holds a
//!    form of any test secret, in the old app before it is dropped and in
//!    the fresh one;
//! 3. the same with an ordinary lock.
//!
//! The rows are a plain list of (name, plant function), `rows!` below.

use crate::common::{Disk, lock, type_text};
use crate::forms::{self, PHRASE};
use crate::{arm, finish};
use faraday_core::convert::ConvertAction as C;
use faraday_core::kef_tool::{KefToolAction, Pick};
use faraday_core::qr_tool::{Field, QrToolAction as Q};
use faraday_core::seed_check::SeedAction as S;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Ask, Faraday, Screen, StorageEvent, catalog, testkit};
use faraday_leak_scan::Form;
use faraday_shell_api::{App, EntropyBytes, Event, Key};

/// Typed into the places that take any text.
const MARKER: &str = "Obsidian-Falcon-Truffle-6620-Juniper-Walrus-Cinder-81";
/// Dice rolled for a vault's passphrase.
const ROLLS: &str = "6125436152436415243651423615243165432165";
/// A compressed mainnet WIF (Bitcoin Core's key_io_valid.json).
const WIF: &str = "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn";
/// BIP 93's vector 2, share A.
const CODEX32_A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";

/// The first part of the test seed's words as a BBQr transfer.
fn first_part() -> String {
    let parts = faraday_qr::bbqr::encode('U', PHRASE.as_bytes(), 60).expect("parts");
    assert!(parts.len() > 1, "the transfer is in parts");
    String::from(parts[0].as_str())
}

/// Every form of every test secret the rows plant.
fn all_forms() -> Vec<Form> {
    let mut f = forms::seed_forms("");
    f.extend(forms::bip85_child_forms());
    const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    f.push(Form::new(
        "codex32 share A data symbols",
        CODEX32_A[3..]
            .bytes()
            .map(|b| CHARSET.iter().position(|&c| c == b).expect("bech32") as u8)
            .collect(),
    ));
    for (name, text) in [
        ("typed text", MARKER.to_string()),
        ("dice rolls", ROLLS.to_string()),
        ("WIF", WIF.to_string()),
        ("codex32 share A", CODEX32_A.to_string()),
        ("codex32 share A after ms1", CODEX32_A[3..].to_string()),
        ("SeedQR digits", forms::seedqr_text()),
        ("TOTP setup code", forms::TOTP_URI.to_string()),
        ("BBQr part", first_part()),
    ] {
        f.push(Form::new(name, text.into_bytes()));
    }
    f
}

fn open_tile(app: &mut Faraday, name: &str) {
    app.press(Action::Nav(Screen::Catalog));
    let at = catalog::TILES
        .iter()
        .position(|t| t.name == name)
        .unwrap_or_else(|| panic!("no tile {name}"));
    app.press(Action::Catalog(at as u8));
}

fn load_seed(app: &mut Faraday) {
    app.session = faraday_core::wallet::Session::on(testkit::NET);
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.press(Action::Entry(None));
    type_text(app, PHRASE);
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
}

fn tick_after_frame(app: &mut Faraday) {
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(app);
}

fn krux_code(app: &mut Faraday) {
    open_tile(app, "Krux code");
    app.press(Action::KefTool(KefToolAction::Pick(Pick::Type)));
    type_text(app, MARKER);
}

fn core_wallet(app: &mut Faraday) {
    open_tile(app, "Read a Bitcoin Core wallet");
    app.press(Action::CoreWallet(
        faraday_core::core_wallet::tool::CoreWalletAction::Focus,
    ));
    type_text(app, MARKER);
}

fn qr_tool(app: &mut Faraday) {
    open_tile(app, "QR vault");
    app.press(Action::QrTool(Q::Pick(faraday_core::qr_tool::Pick::Note)));
    type_text(app, MARKER);
    app.press(Action::QrTool(Q::Next));
    app.press(Action::QrTool(Q::Focus(Field::Pass)));
    type_text(app, MARKER);
}

fn seed_check(app: &mut Faraday) {
    open_tile(app, "My seed makes my wallet");
    app.press(Action::SeedCheck(S::Type));
    testkit::type_words(app, PHRASE);
    app.press(Action::SeedCheck(S::Words));
}

fn lightning(app: &mut Faraday) {
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    type_text(app, PHRASE);
}

fn vanity(app: &mut Faraday) {
    load_seed(app);
    app.press(Action::Vanity(faraday_core::vanity::VanityAction::Open));
}

fn sweep(app: &mut Faraday) {
    open_tile(app, "Sweep a private key");
    type_text(app, WIF);
}

fn calculator(app: &mut Faraday) {
    app.press(Action::Tools);
    app.press(Action::TTool(0));
    type_text(app, MARKER);
}

fn restore_typed_words(app: &mut Faraday) {
    app.press(Action::Entry(None));
    type_text(app, PHRASE);
}

fn codex32_part(app: &mut Faraday) {
    app.press(Action::Entry(None));
    let at = faraday_core::forms::Form::ALL
        .iter()
        .position(|f| *f == faraday_core::forms::Form::Codex32)
        .expect("the codex32 form");
    app.press(Action::EntryForm(at as u8));
    type_text(app, CODEX32_A);
    app.event(Event::Key(Key::Enter));
    testkit::pause(app);
}

fn vault_passphrases(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    type_text(app, MARKER);
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(app, MARKER);
}

fn vault_dice(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::Dice(0)));
    type_text(app, ROLLS);
}

fn read(app: &mut Faraday, code: &[u8]) {
    app.event(Event::Scanned {
        bytes: code.to_vec(),
    });
    let _ = app.frame();
}

fn totp_carried_to_unlock(app: &mut Faraday) {
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("vault.kdbx"),
            testkit::test_vault().unwrap(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.event(Event::Entropy(EntropyBytes::new([7; 32])));
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::ScanFor(Ask::SpendPsbt));
    read(app, forms::TOTP_URI.as_bytes());
    app.press(Action::ReadGo(0));
    assert_eq!(app.screen, Screen::Unlock, "the code was not carried");
}

fn card_under_pull_sheet(app: &mut Faraday) {
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::ScanFor(Ask::SpendPsbt));
    read(app, forms::seedqr_text().as_bytes());
    assert!(app.scan().is_some_and(|s| s.card.is_some()), "no card");
    app.storage(StorageEvent::Sticks(vec![
        Disk::empty("s1", "STICK").info(),
    ]));
    app.press(Action::ReadGo(0));
}

fn transfer_in_parts(app: &mut Faraday) {
    app.press(Action::Scan);
    read(app, first_part().as_bytes());
    app.press(Action::Cancel);
}

fn convert_a_seed(app: &mut Faraday) {
    load_seed(app);
    open_tile(app, "Convert a seed");
    app.press(Action::Convert(C::To(0)));
    tick_after_frame(app);
    app.press(Action::Convert(C::Show));
    let _ = app.frame();
}

fn bip85_child(app: &mut Faraday) {
    load_seed(app);
    app.press(Action::Bip85);
    app.press(Action::PApp(0));
    for _ in 0..4 {
        app.press(Action::PLength(-1));
    }
    app.press(Action::PNext);
    app.event(Event::Key(Key::Char('7')));
    app.press(Action::PNext);
}

fn spend_from_a_backup(app: &mut Faraday) {
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::Entry(None));
    type_text(app, PHRASE);
    app.press(Action::EntryAdd);
}

/// How the row's lock goes.
#[derive(Clone, Copy)]
enum Lock {
    /// The save fails: a fresh app is built over what the old one carries.
    Unkept,
    Ordinary,
}

fn run(name: &str, plant: fn(&mut Faraday), how: Lock) {
    let forms = all_forms();
    let armed = arm(&forms);
    let mut app = testkit::started();
    plant(&mut app);
    assert!(testkit::secret_typed(&app), "{name}: not counted as typed");
    assert!(!app.clean(), "{name}: counted clean while it is held");
    let planted = armed.scan_live();
    assert!(!planted.is_empty(), "{name}: the plant holds no form");
    lock(&mut app);
    match how {
        Lock::Ordinary => {
            let live = armed.scan_live();
            drop(app);
            assert!(live.is_empty(), "{name}: live after Lock: {live:?}");
            finish(name, armed, true);
        }
        Lock::Unkept => {
            let carried = app.carry_unkept_lock();
            let old = armed.scan_live();
            assert!(old.is_empty(), "{name}: old app live after Lock: {old:?}");
            drop(app);
            let mut fresh = testkit::new_on_test_net();
            fresh.start_unrestarted(carried);
            let _ = fresh.frame();
            assert!(
                fresh.session.keys.is_empty(),
                "{name}: the fresh app holds a key"
            );
            finish(name, armed, true);
        }
    }
}

/// The rows: (name, plant function). Two tests a row, so a row that
/// fails alone can be ignored alone.
macro_rules! rows {
    ($($(#[$m:meta])* $id:ident = $plant:ident;)*) => {
        $(
            mod $id {
                use super::*;
                $(#[$m])*
                #[test]
                fn a_lock_whose_save_failed_leaves_no_copy_in_either_app() {
                    run(stringify!($id), $plant, Lock::Unkept);
                }
                $(#[$m])*
                #[test]
                fn an_ordinary_lock_leaves_no_copy() {
                    run(stringify!($id), $plant, Lock::Ordinary);
                }
            }
        )*
    };
}

rows! {
    krux_code_tool = krux_code;
    bitcoin_core_wallet_tool = core_wallet;
    qr_tool = qr_tool;
    seed_check = seed_check;
    lightning = lightning;
    vanity = vanity;
    sweep = sweep;
    #[ignore = "counted from the one rule, not yet built"]
    calculator = calculator;
    restore_typed_words = restore_typed_words;
    codex32_part = codex32_part;
    #[ignore = "counted from the one rule, not yet built"]
    new_vault_passphrases = vault_passphrases;
    #[ignore = "the dice rolls of Create a vault stay in a live block of 512 bytes after Lock, in the old app"]
    new_vault_dice_rolls = vault_dice;
    totp_setup_code_carried_to_unlock = totp_carried_to_unlock;
    read_card_under_the_pull_sheet = card_under_pull_sheet;
    transfer_kept_in_parts = transfer_in_parts;
    #[ignore = "counted from the one rule, not yet built"]
    convert_a_seed = convert_a_seed;
    #[ignore = "counted from the one rule, not yet built"]
    bip85_child = bip85_child;
    spend_from_a_backup = spend_from_a_backup;
}
