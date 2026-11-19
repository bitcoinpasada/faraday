//! The leak tour by area (`docs/AT-LOCK.md` §7, `docs/SECRET-LEAKS.md`
//! §4). [`flows`] matches on [`Area`] with no wildcard arm, so a new area
//! does not compile here until it has a flow, or says it holds no secret.
//! A flow plants a test secret in its area through the app's own input
//! and then:
//!
//! 1. the app is not counted clean while it is held, and the idle warning
//!    comes;
//! 2. Lock, the save failed (the old app is moved to a fresh one in the
//!    same process, `Faraday::carry_unkept_lock`): no live block holds a
//!    form of any test secret, in the old app before it is dropped and in
//!    the fresh one;
//! 3. the same with an ordinary lock.

use crate::common::{Disk, lock, type_text};
use crate::forms::{self, PHRASE};
use crate::{arm, finish};
use faraday_core::convert::ConvertAction as C;
use faraday_core::kef_tool::{KefToolAction, Pick};
use faraday_core::seed_check::SeedAction as S;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Area, Ask, Faraday, Screen, StorageEvent, catalog, testkit};
use faraday_leak_scan::Form;
use faraday_shell_api::{App, EntropyBytes, Event, Key};
use zeroize::Zeroizing;

/// Typed into the places that take any text.
const MARKER: &str = "Obsidian-Falcon-Truffle-6620-Juniper-Walrus-Cinder-81";
/// Dice rolled for a vault's passphrase.
const ROLLS: &str = "6125436152436415243651423615243165432165";
/// A compressed mainnet WIF (Bitcoin Core's key_io_valid.json).
const WIF: &str = "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn";
/// BIP 93's vector 2, share A.
const CODEX32_A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";

/// The bytes of the test seed's words a BBQr part holds.
const PART_BYTES: usize = 60;

/// The first part of the test seed's words as a BBQr transfer, wiped
/// when dropped, as a scanner's own copy would be.
fn first_part() -> Zeroizing<String> {
    let parts = faraday_qr::bbqr::encode('U', PHRASE.as_bytes(), PART_BYTES).expect("parts");
    assert!(parts.len() > 1, "the transfer is in parts");
    Zeroizing::new(String::from(parts[0].as_str()))
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
        ("BBQr part", String::from(first_part().as_str())),
    ] {
        f.push(Form::new(name, text.into_bytes()));
    }
    // What a part kept while the rest is awaited holds: its data read.
    f.push(Form::new(
        "BBQr part's data",
        PHRASE.as_bytes()[..PART_BYTES].to_vec(),
    ));
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

fn secret_sheet(app: &mut Faraday) {
    load_seed(app);
    app.press(Action::Lightning);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::LKey(fp));
    app.press(Action::LOut);
    app.press(Action::SecretUnprotected);
    app.press(Action::SecretHold);
}

fn inbox_passphrase(app: &mut Faraday) {
    let mut file = Zeroizing::new(Vec::with_capacity(PHRASE.len() + 1));
    file.extend_from_slice(PHRASE.as_bytes());
    file.push(b'\n');
    testkit::inbox_file(app, "words.txt", file.to_vec());
    let c = testkit::compute();
    let fp = app.inbox_found_now(&c).potential()[0].fingerprint;
    app.press(Action::PotentialOpen(fp.0));
    app.press(Action::PotentialType);
    type_text(app, MARKER);
}

fn create_passphrase(app: &mut Faraday) {
    load_seed(app);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::CreateWallet);
    crate::common::ckind(app, 0);
    app.press(Action::CNext(faraday_core::cstep::KIND));
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CPassOpen(0));
    type_text(app, MARKER);
    app.press(Action::CPassField(1));
    type_text(app, MARKER);
}

fn keygen_passphrase(app: &mut Faraday) {
    app.press(Action::CreateWallet);
    crate::common::ckind(app, 0);
    app.press(Action::CNext(faraday_core::cstep::KIND));
    app.press(Action::KWords(12));
    for _ in 0..400 {
        if app.keygen().is_some_and(|k| k.ready()) {
            break;
        }
        let before = app.keygen().map_or(0, |k| k.dice.len());
        for f in 1..=6u8 {
            app.press(Action::KRoll(f));
            if app.keygen().is_some_and(|k| k.dice.len() > before) {
                break;
            }
        }
    }
    app.press(Action::KNext);
    app.press(Action::KPassField(0));
    type_text(app, MARKER);
    app.press(Action::KPassField(1));
    type_text(app, MARKER);
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
    read(app, Zeroizing::new(forms::seedqr_text()).as_bytes());
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
    app.press(Action::PShow);
    let _ = app.frame();
}

fn spend_from_a_backup(app: &mut Faraday) {
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::Entry(None));
    type_text(app, PHRASE);
    app.press(Action::EntryAdd);
}

/// How the flow's secret is counted while it is held.
#[derive(Clone, Copy)]
enum Held {
    /// On its way: typed, scanned or shown in a tool.
    Typed,
    /// A key the session keeps.
    Kept,
}

/// How the lock goes.
#[derive(Clone, Copy)]
enum Lock {
    /// The save fails: a fresh app is built over what the old one carries.
    Unkept,
    Ordinary,
}

/// One flow of an area: its name, what plants the secret and how it is
/// counted.
#[derive(Clone, Copy)]
struct Flow {
    name: &'static str,
    plant: fn(&mut Faraday),
    held: Held,
    /// Whether the idle warning comes: not while Make a new wallet holds
    /// the idle lock until its keys are kept somewhere.
    warns: bool,
}

const fn typed(name: &'static str, plant: fn(&mut Faraday)) -> Flow {
    Flow {
        name,
        plant,
        held: Held::Typed,
        warns: true,
    }
}

/// A flow in Make a new wallet, which holds the idle lock.
const fn typed_in_make(name: &'static str, plant: fn(&mut Faraday)) -> Flow {
    Flow {
        name,
        plant,
        held: Held::Typed,
        warns: false,
    }
}

const fn kept(name: &'static str, plant: fn(&mut Faraday)) -> Flow {
    Flow {
        name,
        plant,
        held: Held::Kept,
        warns: true,
    }
}

/// The flows that cover an area. An area that holds no secret of its own
/// has none, and its arm says why.
fn flows(area: Area) -> Vec<Flow> {
    match area {
        // What frames keep: always filled, public descriptions and glyphs.
        Area::Canvas | Area::Drawing | Area::Kept => vec![],
        Area::Tools => vec![
            typed("Krux code tool", krux_code),
            typed("Bitcoin Core wallet tool", core_wallet),
            typed("seed check", seed_check),
            typed("calculator", calculator),
            typed("Convert a seed", convert_a_seed),
        ],
        Area::Lightning => vec![typed("Lightning", lightning)],
        Area::Vanity => vec![typed("Vanity", vanity)],
        Area::SeedRead => vec![typed("Sweep", sweep)],
        Area::Entry => vec![
            typed("restore's typed words", restore_typed_words),
            typed("a codex32 part", codex32_part),
        ],
        Area::Vaults => vec![
            typed("new vault's passphrases", vault_passphrases),
            typed("new vault's dice rolls", vault_dice),
        ],
        Area::Camera => vec![
            typed("TOTP setup code carried to Unlock", totp_carried_to_unlock),
            typed("read card under the Pull sheet", card_under_pull_sheet),
            typed("transfer kept in parts", transfer_in_parts),
        ],
        Area::Bip85 => vec![typed("BIP-85 child", bip85_child)],
        Area::SpendBackup => vec![kept("Spend from a backup", spend_from_a_backup)],
        Area::Session => vec![kept("a key loaded", load_seed)],
        Area::Secrets => vec![typed("a secret on its sheet", secret_sheet)],
        Area::Inbox => vec![typed("a seed file's passphrase", inbox_passphrase)],
        Area::Create => vec![typed_in_make(
            "a key's passphrase in Make a new wallet",
            create_passphrase,
        )],
        Area::KeyGen => vec![typed_in_make(
            "a passphrase on the Keys card after rolls",
            keygen_passphrase,
        )],
        // A word looked up is one word of a public list, which §3 of
        // `docs/SECRET-LEAKS.md` never searches for alone.
        Area::WordList => vec![],
        // Randomness the person added is mixed with the system's and is not
        // a secret alone.
        Area::Fresh => vec![],
        // Reads of the session's own keys, or public contents: a message and
        // its signature, a code on show, an explorer's lines, a decode check,
        // a spend's PSBT, a stick visit's name question, the files the boot
        // stick brought, and Faraday Demo's transfer text.
        Area::Decode
        | Area::Explore
        | Area::Qr
        | Area::Message
        | Area::Wallets
        | Area::Visit
        | Area::Import => vec![],
        #[cfg(feature = "demo")]
        Area::Transfer => vec![],
        // No flow yet: the passphrase typed on the At a glance sheet, restore's
        // typed shares, the backup flow's copy typed back, the GPG restore's
        // typed lines and a Krux code opened for a screen.
        Area::Glance | Area::Restore | Area::Backup | Area::Flows | Area::Worker => vec![],
    }
}

fn idle_warns(app: &mut Faraday, name: &str) {
    let from = app.now_ms();
    app.event(Event::Tick { now_ms: from });
    app.event(Event::Tick {
        now_ms: from + 26 * 60_000,
    });
    assert_eq!(
        app.sheet,
        Some(faraday_core::Sheet::IdleWarn),
        "{name}: no idle warning"
    );
}

fn run(flow: Flow, how: Lock) {
    let Flow {
        name,
        plant,
        held,
        warns,
    } = flow;
    let forms = all_forms();
    let armed = arm(&forms);
    let mut app = testkit::started();
    plant(&mut app);
    match held {
        Held::Typed => assert!(testkit::secret_typed(&app), "{name}: not counted as typed"),
        Held::Kept => assert!(app.holds_secret(), "{name}: not counted as kept"),
    }
    assert!(!app.clean(), "{name}: counted clean while it is held");
    let planted = armed.scan_live();
    assert!(!planted.is_empty(), "{name}: the plant holds no form");
    if warns {
        idle_warns(&mut app, name);
    }
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

/// Every flow of every area, locked the way `how` says.
fn every_area(how: Lock) {
    for &area in Area::ALL {
        for flow in flows(area) {
            run(flow, how);
        }
    }
}

#[test]
fn a_lock_whose_save_failed_leaves_no_copy_in_either_app() {
    every_area(Lock::Unkept);
}

#[test]
fn an_ordinary_lock_leaves_no_copy() {
    every_area(Lock::Ordinary);
}
