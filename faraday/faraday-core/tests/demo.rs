//! The demo (F30, `docs/DEMO.md`): every screen opens, and every action
//! that would make a secret a person could take away is dimmed and does
//! nothing when pressed. New key and Generate with dice run, and what
//! they make is replaced by a public value. Bitcoin use with the public
//! seeds stays open. On a test network, and on the stick, every one of
//! those actions works.

mod common;

use faraday_core::backup_flow::Kind;
use faraday_core::backup_flow_steps::FlowAction as F;
use faraday_core::create::NewKind;
use faraday_core::keygen::{MIX_SOURCES, Source, Way, kstep};
use faraday_core::plan::{Item, Preset};
use faraday_core::seeds::SeedsAction as S;
use faraday_core::testkit::{self, Kit};
use faraday_core::vanity::VanityAction as VA;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, bstep, cstep, demo, qstep};
use faraday_vault::records::kind;
use opensigner_core::vanity::Dial;
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::Network;
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

/// The demo's Learn page.
const DEMO_PAGE: &str = "The demo and its public seeds";

/// The passphrase of the empty vault in Files.
const VAULT_PHRASE: &str = "demo vault";

/// An empty vault, in Files from the start.
fn vault_file() -> (String, Vec<u8>) {
    (
        "demo.ofv".to_string(),
        common::vault("Demo", &[VAULT_PHRASE], 9),
    )
}

/// A display tall enough that an open card is drawn whole.
fn tall(app: &mut Faraday) {
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2400,
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

/// The app with the test vault in Files and the clock checked: as the
/// demo has it (online, on mainnet, the public seeds set up), or as the
/// stick has it.
fn app_on(demo: bool) -> Faraday {
    let mut app = common::process(vec![vault_file()], Vec::new(), Vec::new());
    if demo {
        app.online = true;
        app.session.set_demo();
    }
    common::keep_time(&mut app);
    tall(&mut app);
    app
}

/// The vault in Files, unlocked.
fn unlock(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    common::type_text(app, VAULT_PHRASE);
    app.press(Action::Vault(V::Unlock));
    common::settle(app, 1);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
}

/// A public seed typed into Add a key. Returns its fingerprint.
fn add_public(app: &mut Faraday, word: &str) -> [u8; 4] {
    app.press(Action::Entry(None));
    common::type_text(app, &demo::words(word));
    app.press(Action::EntryAdd);
    public_fp(word)
}

/// A public seed's fingerprint.
fn public_fp(word: &str) -> [u8; 4] {
    let mut s = faraday_core::wallet::Session::default();
    s.add_words(&demo::words(word), "", None).unwrap().0
}

/// A key loaded: bacon, typed in unless it is loaded already.
fn a_key(app: &mut Faraday) -> [u8; 4] {
    let fp = public_fp("bacon");
    if !app.session.keys.iter().any(|k| k.fingerprint().0 == fp) {
        add_public(app, "bacon");
    }
    fp
}

fn records(app: &Faraday, k: u8) -> usize {
    app.vaults.open[app.vaults.current].contents.of(k).count()
}

/// Whether the frame drawn now offers `a`.
fn offered(app: &mut Faraday, a: Action) -> bool {
    common::offered(app, a)
}

fn shown(app: &mut Faraday, text: &str) -> bool {
    common::has(app, text)
}

// ---------------------------------------------------------------------
// GPG, Secure Boot and APK keys: the flow opens, Make does nothing.
// ---------------------------------------------------------------------

/// Starts `kind`'s flow from its Tools tile, then stands on Make over the
/// open vault and presses Make, by the button and by the form's Enter.
/// Returns whether the screen opened, whether Make was offered, and
/// how many keys of the kind the vault gained and files For the stick.
fn make_from_tools(app: &mut Faraday, kind_: Kind, fields: &[&str]) -> (bool, bool, usize, usize) {
    let rec = match kind_ {
        Kind::Gpg => kind::GPG,
        Kind::SecureBoot => kind::SECURE_BOOT,
        _ => kind::APK_KEY,
    };
    common::start(app, kind_);
    let opened = app.screen == Screen::BackupFlow;
    unlock(app);
    let (keys, files) = (records(app, rec), app.outbox.len());
    common::at_make(app, kind_);
    let make = offered(app, Action::Flow(F::Make));
    common::make_in_form(app, fields);
    app.press(Action::Vault(V::FormSave));
    common::settle_n(app, 22, 2);
    (
        opened,
        make,
        records(app, rec) - keys,
        app.outbox.len() - files,
    )
}

#[test]
fn in_the_demo_the_gpg_key_screen_opens_and_make_adds_no_key() {
    let mut app = app_on(true);
    let (opened, make, keys, files) = make_from_tools(&mut app, Kind::Gpg, &["Release", ""]);
    assert!(opened, "the GPG key screen opens");
    assert!(shown(&mut app, "Make the key"), "Make is drawn");
    assert!(!make, "Make a GPG key is dimmed");
    assert_eq!((keys, files), (0, 0), "no GPG key, nothing For the stick");
}

#[test]
fn in_the_demo_secure_boot_and_apk_keys_are_not_made() {
    for (k, fields) in [
        (Kind::SecureBoot, &["Owner"][..]),
        (Kind::Apk, &["Faraday Release", "com.example.app"][..]),
    ] {
        let mut app = app_on(true);
        let (opened, make, keys, files) = make_from_tools(&mut app, k, fields);
        assert!(opened, "{k:?} opens");
        assert!(!make, "{k:?}: Make is dimmed");
        assert_eq!((keys, files), (0, 0), "{k:?}: nothing made");
    }
}

#[test]
fn on_the_stick_and_on_a_test_network_make_a_gpg_key_makes_one() {
    let mut app = app_on(false);
    let (opened, make, keys, _) = make_from_tools(&mut app, Kind::Gpg, &["Release", ""]);
    assert!(opened && make);
    assert_eq!(keys, 1, "the stick makes the key");

    // The demo set up, moved to a test network: the demo is off.
    let mut app = app_on(true);
    app.press(Action::Network(Network::Testnet));
    app.press(Action::AirgapUnderstood);
    assert_eq!(app.session.network(), Network::Testnet);
    let (_, make, keys, _) = make_from_tools(&mut app, Kind::Gpg, &["Release", ""]);
    assert!(make);
    assert_eq!(keys, 1, "a test network makes the key");
}

#[test]
fn in_the_demo_the_gpg_key_screen_names_the_demo_page_first() {
    let mut app = app_on(true);
    common::start(&mut app, Kind::Gpg);
    assert_eq!(app.screen, Screen::BackupFlow);
    app.press(Action::Learn);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert_eq!(app.learn_pages().first().map(|p| p.title), Some(DEMO_PAGE));
    // The stick never names it.
    let mut app = app_on(false);
    common::start(&mut app, Kind::Gpg);
    assert!(app.learn_pages().iter().all(|p| p.title != DEMO_PAGE));
}

#[test]
fn on_a_test_network_the_gpg_key_screen_does_not_name_the_demo_page_first() {
    let mut app = app_on(true);
    app.press(Action::Network(Network::Testnet));
    app.press(Action::AirgapUnderstood);
    assert_eq!(app.session.network(), Network::Testnet);
    common::start(&mut app, Kind::Gpg);
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_ne!(app.learn_pages().first().map(|p| p.title), Some(DEMO_PAGE));
}

// ---------------------------------------------------------------------
// Seed XOR, codex32 and the backup plan.
// ---------------------------------------------------------------------

/// A one-key wallet over bacon, its backup's checklist made from Paper
/// only, the copy open and shown.
fn backing_up(app: &mut Faraday) -> usize {
    let fp = a_key(app);
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    let paper = Preset::ALL
        .iter()
        .position(|p| *p == Preset::Paper)
        .unwrap() as u8;
    app.press(Action::BPreset(paper));
    app.press(Action::BQ(qstep::PLAN));
    app.press(Action::BChecklist);
    if app.backup_items().contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    if app.backup.as_ref().unwrap().open != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
    app.press(Action::BReveal);
    w
}

/// Whether any frame down the column offers `a`, scrolling as a person
/// would.
fn offered_anywhere(app: &mut Faraday, a: Action) -> bool {
    for _ in 0..20 {
        app.settle();
        if offered(app, a) {
            return true;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    false
}

const SPLITS: [Action; 4] = [
    Action::BXor(2),
    Action::BXor(3),
    Action::BCodex32(2, 3),
    Action::BCodex32(3, 5),
];

#[test]
fn in_the_demo_seed_xor_and_codex32_shares_are_dimmed_and_make_nothing() {
    let mut app = app_on(true);
    backing_up(&mut app);
    assert!(
        common::has_part(&mut app, "Seed XOR"),
        "the splits are drawn"
    );
    for a in SPLITS {
        assert!(!offered_anywhere(&mut app, a), "{a:?} is offered");
        app.press(a);
        assert!(
            app.backup.as_ref().unwrap().paper.is_none(),
            "{a:?} made parts"
        );
    }
    assert!(app.outbox.iter().all(|i| !i.name.contains("xor")));
}

#[test]
fn on_the_stick_seed_xor_and_codex32_shares_are_made() {
    for a in SPLITS {
        let mut app = app_on(false);
        backing_up(&mut app);
        assert!(offered_anywhere(&mut app, a), "{a:?} is not offered");
        app.press(a);
        assert!(
            app.backup.as_ref().unwrap().paper.is_some(),
            "{a:?} made nothing"
        );
    }
}

#[test]
fn in_the_demo_a_backup_plan_offers_no_split_and_its_checklist_can_be_finished() {
    let mut app = app_on(true);
    let fp = a_key(&mut app);
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    let paper = Preset::ALL
        .iter()
        .position(|p| *p == Preset::Paper)
        .unwrap() as u8;
    app.press(Action::BPreset(paper));
    // The board: none of what it offers or keeps is a split of the seed.
    if app.backup.as_ref().unwrap().q != Some(qstep::BOARD) {
        app.press(Action::BQ(qstep::BOARD));
    }
    let texts = common::texts(&mut app);
    for split in ["Seed XOR", "codex32", "SLIP-39"] {
        assert!(
            texts.iter().all(|t| !t.contains(split)),
            "the board offers {split}: {texts:?}"
        );
    }
    app.press(Action::BQ(qstep::PLAN));
    app.press(Action::BChecklist);
    if app.backup_items().contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    // The copy, typed back.
    if app.backup.as_ref().unwrap().open != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
    app.press(Action::BReveal);
    app.storage(faraday_core::StorageEvent::Cameras(Vec::new()));
    app.press(Action::BCheck);
    let digits = osk_codec::seedqr::to_digits(
        &Mnemonic::parse(Language::English, &demo::words("bacon")).unwrap(),
    );
    common::type_text(
        &mut app,
        std::str::from_utf8(digits.expose().as_bytes()).unwrap(),
    );
    assert!(
        app.backup_item_done(Item::Copy(0)),
        "the copy did not check"
    );
    // The envelopes, then the writes, the last item.
    app.press(Action::BStep(bstep::ENVELOPE));
    app.press(Action::BNext(bstep::ENVELOPE));
    assert!(app.backup_item_done(Item::Envelopes));
    for item in [
        Item::Templates,
        Item::Sheets,
        Item::Copy(0),
        Item::Envelopes,
    ] {
        if app.backup_items().contains(&item_step(item)) {
            assert!(app.backup_item_done(item), "{item:?} is not done");
        }
    }
    assert!(
        offered_anywhere(&mut app, Action::Flow(F::WalletLock)),
        "the writes cannot be started"
    );
}

/// The checklist step an item is drawn on.
fn item_step(item: Item) -> u8 {
    match item {
        Item::Templates => bstep::BLANK,
        Item::Sheets => bstep::SHEETS,
        Item::Copy(_) => bstep::COPY,
        _ => bstep::ENVELOPE,
    }
}

// ---------------------------------------------------------------------
// SLIP-39 shares.
// ---------------------------------------------------------------------

/// New key as SLIP-39 shares from coin flips, 2 of 3, as far as the deal.
fn slip39(app: &mut Faraday) -> (bool, usize) {
    app.press(Action::KeyGenSlip39);
    assert_eq!(app.screen, Screen::KeyGen, "SLIP-39 shares opens");
    app.press(Action::KWords(20));
    if app.keygen.as_ref().unwrap().open == Some(kstep::LENGTH) {
        app.press(Action::KNext);
    }
    assert_eq!(app.keygen.as_ref().unwrap().open, Some(kstep::SOURCE));
    let ways = offered(app, Action::KWay(Way::Coins.index()));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip(i % 3 != 1));
    }
    app.press(Action::KNext);
    (ways, app.keygen.as_ref().map_or(0, |k| k.shares.len()))
}

#[test]
fn in_the_demo_slip39_shares_opens_and_deals_nothing() {
    let mut app = app_on(true);
    let (ways, shares) = slip39(&mut app);
    assert!(!ways, "its sources are dimmed");
    assert_eq!(shares, 0, "no share is dealt");
    assert!(app.session.keys.is_empty());

    let mut app = app_on(false);
    let (ways, shares) = slip39(&mut app);
    assert!(ways);
    assert_eq!(shares, 3, "the stick deals 2 of 3");
}

// ---------------------------------------------------------------------
// BIP-85.
// ---------------------------------------------------------------------

/// BIP-85 on bacon, application `a`, on its Value card.
fn bip85_value(app: &mut Faraday, a: u8) {
    let fp = a_key(app);
    app.press(Action::Bip85);
    assert_eq!(app.screen, Screen::Bip85, "BIP-85 opens");
    app.press(Action::PKey(fp));
    app.press(Action::PApp(a));
    app.press(Action::PNext);
    app.press(Action::PNext);
}

#[test]
fn in_the_demo_derive_with_bip85_shows_no_value_for_any_application() {
    for a in 0..faraday_core::bip85::APPS.len() as u8 {
        let mut app = app_on(true);
        unlock(&mut app);
        bip85_value(&mut app, a);
        let entries = records(&app, kind::ENTRY) + records(&app, kind::NOTE);
        let keys = app.session.keys.len();
        for p in [Action::PShow, Action::PVault, Action::PLoad, Action::POut] {
            assert!(!offered(&mut app, p), "{p:?} offered for app {a}");
            app.press(p);
        }
        assert!(app.bip85_value().is_none(), "a value for app {a}");
        assert!(app.bip85.as_ref().is_some_and(|b| !b.shown));
        assert_eq!(app.sheet, None, "no secret sheet");
        assert_eq!(
            records(&app, kind::ENTRY) + records(&app, kind::NOTE),
            entries
        );
        assert_eq!(app.session.keys.len(), keys, "no child loaded");
        assert!(app.outbox.is_empty());
    }
    // On the stick the value is there to show.
    let mut app = app_on(false);
    bip85_value(&mut app, 0);
    assert!(offered(&mut app, Action::PShow));
    assert!(app.bip85_value().is_some());
}

// ---------------------------------------------------------------------
// FROST, vanity, Lightning, silent payments.
// ---------------------------------------------------------------------

/// Create, FROST, its two chosen shares from public seeds, on Keys.
/// Returns whether Keys' Continue is offered, after pressing it.
fn frost(app: &mut Faraday) -> bool {
    let a = add_public(app, "bacon");
    let b = add_public(app, "flag");
    faraday_core::testkit::create_knowing(app);
    let at = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Threshold)
        .unwrap() as u8;
    app.press(Action::CKind(at));
    app.press(Action::CSlotHere(0, a));
    app.press(Action::CSlotHere(1, b));
    if let Some(c) = app.create.as_mut() {
        c.open = Some(cstep::KEYS);
    }
    let live = offered(app, Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::KEYS));
    live
}

#[test]
fn in_the_demo_the_frost_deal_is_dimmed() {
    let mut app = app_on(true);
    assert!(!frost(&mut app), "Keys' Continue is offered");
    assert!(app.create.as_ref().unwrap().built.is_none(), "dealt");
    assert!(app.session.wallets.is_empty());
    assert_eq!(app.session.keys.len(), 2, "no share loaded");

    let mut app = app_on(false);
    assert!(frost(&mut app));
    assert!(
        app.create.as_ref().unwrap().built.is_some(),
        "{:?}",
        app.create.as_ref().unwrap().error
    );
}

fn passphrase_dial() -> u8 {
    Dial::ALL
        .iter()
        .position(|d| *d == Dial::Passphrase)
        .unwrap() as u8
}

/// The vanity screen on bacon, on its dial card; the passphrase dial
/// pressed. Returns whether it was offered.
fn vanity(app: &mut Faraday) -> bool {
    a_key(app);
    app.press(Action::Vanity(VA::Open));
    assert_eq!(app.screen, Screen::Vanity, "Vanity address opens");
    if let Some(v) = app.vanity.as_mut() {
        v.open = Some(faraday_core::vanity::vstep::DIAL);
    }
    let live = offered(app, Action::Vanity(VA::Dial(passphrase_dial())));
    app.press(Action::Vanity(VA::Dial(passphrase_dial())));
    live
}

#[test]
fn in_the_demo_the_vanity_passphrase_dial_is_dimmed() {
    let mut app = app_on(true);
    assert!(!vanity(&mut app));
    assert_eq!(app.vanity.as_ref().unwrap().grind.dial, Dial::Account);
    assert!(offered(&mut app, Action::Vanity(VA::Dial(0))) || passphrase_dial() == 0);

    let mut app = app_on(false);
    assert!(vanity(&mut app));
    assert_eq!(app.vanity.as_ref().unwrap().grind.dial, Dial::Passphrase);
}

/// Lightning node key from bacon, with the test vault open. Returns
/// which of the aezeed row, Save into and Out unprotected are offered.
fn lightning(app: &mut Faraday) -> [bool; 3] {
    unlock(app);
    let fp = a_key(app);
    app.press(Action::Lightning);
    assert_eq!(app.screen, Screen::Lightning, "Lightning node key opens");
    app.press(Action::LKey(fp));
    assert!(app.lightning.as_ref().unwrap().node.is_some(), "no node id");
    [Action::LAezeed, Action::LVault, Action::LOut].map(|a| offered(app, a))
}

#[test]
fn in_the_demo_a_lightning_node_key_stays_in_faraday() {
    let mut app = app_on(true);
    assert_eq!(lightning(&mut app), [false; 3]);
    // Show is dimmed too, and pressing it puts no private key on screen.
    assert!(!offered(&mut app, Action::LShow), "Show is offered");
    let files = app.outbox.len();
    app.press(Action::LShow);
    assert!(
        !app.lightning.as_ref().unwrap().shown,
        "the private key is shown"
    );
    let private = {
        let n = app.lightning.as_ref().unwrap().node.as_ref().unwrap();
        n.private
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    assert!(
        !common::texts(&mut app).iter().any(|t| t.contains(&private)),
        "the private key is drawn"
    );
    assert_eq!(app.outbox.len(), files, "something went For the stick");
    let notes = records(&app, kind::NOTE);
    for a in [Action::LVault, Action::LOut] {
        app.press(a);
    }
    assert_eq!(records(&app, kind::NOTE), notes, "saved into the vault");
    assert_eq!(app.sheet, None, "no secret sheet");
    app.press(Action::LAezeed);
    assert!(
        app.lightning.as_ref().unwrap().node.is_some(),
        "the loaded key's node stays"
    );

    let mut app = app_on(false);
    assert_eq!(lightning(&mut app), [true; 3]);
    assert!(offered(&mut app, Action::LShow));
}

/// Silent payments on bacon, on the scan key card, the test vault open.
/// Returns whether Save into and Out unprotected are offered.
fn scan_key(app: &mut Faraday) -> [bool; 2] {
    unlock(app);
    let fp = a_key(app);
    app.press(Action::Silent);
    assert_eq!(app.screen, Screen::Silent, "Silent payments opens");
    app.press(Action::SKey(fp));
    app.press(Action::SNext);
    assert_eq!(
        app.silent.as_ref().unwrap().open,
        Some(faraday_core::silent::sstep::SCAN)
    );
    [Action::SScanVault, Action::SScanOut].map(|a| offered(app, a))
}

#[test]
fn in_the_demo_a_scan_key_stays_in_faraday() {
    let mut app = app_on(true);
    assert_eq!(scan_key(&mut app), [false; 2]);
    let notes = records(&app, kind::NOTE);
    app.press(Action::SScanVault);
    app.press(Action::SScanOut);
    assert_eq!(records(&app, kind::NOTE), notes);
    assert_eq!(app.sheet, None);
    assert!(app.outbox.is_empty());

    let mut app = app_on(false);
    assert_eq!(scan_key(&mut app), [true; 2]);
}

// ---------------------------------------------------------------------
// New key: every source runs; the next public seed is added.
// ---------------------------------------------------------------------

/// One entry for the source being entered.
fn feed(app: &mut Faraday, i: usize) {
    let Some(k) = app.keygen.as_ref() else {
        return;
    };
    match k.active() {
        Some(Source::Dice) => app.press(Action::KRoll((i % 4) as u8 + 1)),
        Some(Source::Coins) => app.press(Action::KFlip((i * 5 + i / 3).is_multiple_of(2))),
        Some(Source::Hex) => app.press(Action::KHex((i * 7 % 16) as u8)),
        Some(Source::Cards) => {
            let c = (i % 52) as u8;
            app.press(Action::KRank(c % 13));
            app.press(Action::KSuit(c / 13));
        }
        Some(Source::Camera) => {
            let luma: Vec<u8> = (0..1024usize)
                .map(|p| ((p * 31 + i * 17 + p / 7) % 256) as u8)
                .collect();
            app.event(Event::CameraFrame {
                width: 32,
                height: 32,
                luma,
                chroma: None,
            });
            app.press(Action::KFrame);
        }
        Some(Source::Device) => {
            app.event(Event::Entropy(EntropyBytes::new([i as u8 ^ 0x3c; 32])));
            app.press(Action::KNext);
        }
        _ => app.press(Action::KNext),
    }
}

/// New key from `way` at `words` words, every entry made, on the Key
/// card. Returns the words made.
fn new_key(app: &mut Faraday, way: Way, words: u8) -> String {
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    app.press(Action::KWords(words));
    app.press(Action::KNext);
    app.press(Action::KWay(way.index()));
    if way == Way::Mix {
        let pos = |s: Source| MIX_SOURCES.iter().position(|m| *m == s).unwrap() as u8;
        app.press(Action::KMix(pos(Source::Dice)));
        app.press(Action::KMix(pos(Source::Coins)));
    }
    app.press(Action::KNext);
    for i in 0..3000 {
        let k = app.keygen.as_ref().unwrap();
        if k.open == Some(kstep::KEY) {
            break;
        }
        let (have, need) = k.progress();
        if have < need {
            feed(app, i);
        } else {
            app.press(Action::KNext);
        }
    }
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.open, Some(kstep::KEY), "{way:?} at {words}: {:?}", k.note);
    k.phrase().expect("words made").to_string()
}

fn fingerprint(words: &str) -> [u8; 4] {
    let mut s = faraday_core::wallet::Session::default();
    s.add_words_with(words, "", "", None).unwrap().0
}

fn loaded(app: &Faraday) -> Vec<[u8; 4]> {
    app.session.keys.iter().map(|k| k.fingerprint().0).collect()
}

/// Lock in, the words' quiz passed, Done.
fn finish(app: &mut Faraday) {
    use opensigner_core::quiz::QuizState;
    app.press(Action::KLock);
    app.press(Action::KNext);
    app.press(Action::KNext);
    for _ in 0..30 {
        let Some(q) = app.keygen.as_ref().and_then(|k| k.quiz.as_ref()) else {
            break;
        };
        if q.state() == QuizState::Passed {
            break;
        }
        let slot = q.correct_slot() as u8;
        app.press(Action::KQuiz(slot));
    }
    app.press(Action::KAdd);
}

#[test]
fn in_the_demo_new_key_from_every_source_adds_bacon_in_place_of_its_words() {
    let ways = [
        Way::DiceFlips,
        Way::Coins,
        Way::DiceWords,
        Way::DiceHashed,
        Way::DiceSixAsZero,
        Way::Hex,
        Way::Cards,
        Way::Camera,
        Way::Mix,
        Way::Device,
    ];
    for words in [12u8, 24] {
        for way in ways {
            let mut app = app_on(true);
            let made = new_key(&mut app, way, words);
            assert_eq!(made.split(' ').count(), usize::from(words));
            assert_ne!(made, demo::words("bacon"));
            assert!(
                common::has_part(&mut app, "Lock in adds public seed bacon"),
                "{way:?} at {words}: the Key card does not say so"
            );
            finish(&mut app);
            let have = loaded(&app);
            assert_eq!(have, vec![public_fp("bacon")], "{way:?} at {words}");
            assert!(!have.contains(&fingerprint(&made)), "the key made was kept");
        }
    }
}

#[test]
fn in_the_demo_with_every_public_seed_loaded_lock_in_is_dimmed() {
    let mut app = app_on(true);
    for w in demo::SEEDS {
        app.session.add_words(&demo::words(w), "", None).unwrap();
    }
    new_key(&mut app, Way::Device, 12);
    assert!(!offered(&mut app, Action::KLock), "Lock in is offered");
    app.press(Action::KLock);
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.session.keys.len(), demo::SEEDS.len(), "a key was added");
    assert!(!app.keygen.as_ref().unwrap().locked);
}

// ---------------------------------------------------------------------
// Create a vault: Generate with dice and a typed passphrase.
// ---------------------------------------------------------------------

/// Thirty dice: six words of the long list.
const ROLLS: &str = "123451234512345123451234512345";

/// Create a vault from Vaults, its first passphrase from the dice or
/// typed, made and left on Unlock with it picked.
fn make_vault(app: &mut Faraday, typed: Option<&str>) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    assert_eq!(app.screen, Screen::CreateVault);
    match typed {
        None => {
            app.press(Action::Vault(V::Dice(0)));
            common::type_text(app, ROLLS);
            app.press(Action::Vault(V::DiceUse));
        }
        Some(p) => {
            app.press(Action::Vault(V::CFocus(0, false)));
            common::type_text(app, p);
            app.press(Action::Vault(V::CFocus(0, true)));
            common::type_text(app, p);
        }
    }
    let _ = common::texts(app);
    app.press(Action::Vault(V::CGo));
    common::settle(app, 1);
    assert_eq!(app.screen, Screen::Unlock, "the vault was made");
}

/// Unlock with `phrase`. Returns whether the vault opened.
fn opens_with(app: &mut Faraday, phrase: &str) -> bool {
    let before = app.vaults.open.len();
    common::unlock(app, phrase);
    app.vaults.open.len() > before
}

#[test]
fn in_the_demo_a_vault_from_the_dice_opens_with_the_public_passphrase() {
    let rolled = faraday_core::vaults::dice_words(ROLLS, osk_bip::diceware::List::Large).join("");
    let mut app = app_on(true);
    make_vault(&mut app, None);
    assert!(!opens_with(&mut app, &rolled), "the rolled words open it");
    let mut app = app_on(true);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::Dice(0)));
    common::type_text(&mut app, ROLLS);
    app.press(Action::Vault(V::DiceUse));
    assert!(common::has(
        &mut app,
        "The demo uses its public passphrase in place of these words"
    ));
    app.press(Action::Vault(V::CGo));
    common::settle(&mut app, 1);
    assert!(opens_with(&mut app, demo::PASSPHRASE));

    // On the stick the rolled words are the passphrase.
    let mut app = app_on(false);
    make_vault(&mut app, None);
    assert!(opens_with(&mut app, &rolled));
}

#[test]
fn in_the_demo_a_vault_with_a_typed_passphrase_opens_with_it() {
    let mut app = app_on(true);
    make_vault(&mut app, Some("harbour lantern"));
    assert!(opens_with(&mut app, "harbour lantern"));
}

// ---------------------------------------------------------------------
// What stays open.
// ---------------------------------------------------------------------

#[test]
fn in_the_demo_public_seeds_sign_keep_entries_and_go_into_a_vault() {
    let mut app = app_on(true);
    // A wallet from bacon signs a transaction.
    let fp = a_key(&mut app);
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    let w = app.session.wallets.len() - 1;
    let kit = Kit {
        id: "demo",
        name: "Demo",
        descriptor: app.session.wallets[w].policy().to_descriptor_checksummed(),
        cosigner: None,
        fee: 1_000,
    };
    let psbt = testkit::unsigned(&kit).unwrap();
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: vec![vault_file(), ("spend.psbt".to_string(), psbt.to_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = app
        .inbox
        .iter()
        .position(|i| i.name == "spend.psbt")
        .unwrap();
    app.press(Action::StartSpend(at));
    app.press(Action::SignHere);
    let s = app.spend.as_ref().unwrap();
    assert!(s.spend.finished.is_some(), "not signed: {:?}", s.error);

    // A message.
    app.press(Action::SignMessage);
    assert_eq!(app.screen, Screen::Message, "Sign a message opens");
    app.message.as_mut().unwrap().text = "Faraday signs this".to_string();
    app.press(Action::MSign);
    assert!(
        app.message.as_ref().unwrap().signed.is_some(),
        "the message is not signed"
    );

    // A vault with a typed passphrase, made and unlocked; an entry with
    // its TOTP code; the public seed saved into it.
    make_vault(&mut app, Some("harbour lantern"));
    assert!(opens_with(&mut app, "harbour lantern"));
    app.press(Action::OpenCodes);
    app.press(Action::CodesAdd);
    app.press(Action::Vault(V::FocusField(0)));
    common::type_text(&mut app, "New site");
    app.press(Action::Vault(V::FocusField(5)));
    common::type_text(&mut app, "JBSW Y3DP EHPK 3PXP");
    app.press(Action::Vault(V::FormSave));
    let now = app.clock_now().unwrap();
    let rows = faraday_core::codes::codes_rows(&app, now);
    let new = rows
        .iter()
        .find(|r| r.title == "New site")
        .expect("the entry");
    assert!(matches!(
        new.kind,
        faraday_core::codes::CodeKind::Code { .. }
    ));
    let keys = records(&app, kind::KEY);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(0)));
    app.press(Action::Vault(V::Add));
    app.press(Action::Vault(V::SaveKey(0)));
    assert_eq!(
        records(&app, kind::KEY),
        keys + 1,
        "the seed is in the vault"
    );
}
