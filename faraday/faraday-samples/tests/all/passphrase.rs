//! Every passphrase is `faraday` in Demo (`docs/MODES.md` §6.2, §10 M8): a
//! passphrase field shows it in a code box, takes no typing, and the flow
//! goes on with it; outside Demo every field is typed as before. The
//! tests drive a `Faraday` with the real samples set, as the desktop does.

use crate::common::{demo_app, show};
use faraday_bip::keys::{Fingerprint, Network};
use faraday_core::catalog::{Go, TILES};
use faraday_core::explore::{Page, XAction};
use faraday_core::lightning::NodeSource;
use faraday_core::samples::{Pick, SampleForm};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_samples::SAMPLES;
use faraday_shell_api::{App, Event, Key};

/// The sample made of `word`.
fn words_of(word: &str) -> &'static faraday_core::samples::SampleKey {
    SAMPLES
        .keys
        .iter()
        .find(|k| k.word == word)
        .unwrap_or_else(|| panic!("no sample of {word}"))
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Frames and ticks, a second apart.
fn settle(app: &mut Faraday) {
    for t in 1..=30u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// Whether the frame drawn now has `text` as one of its strings.
fn shows(app: &mut Faraday, text: &str) -> bool {
    app.drawn_texts().iter().any(|t| t == text)
}

/// Whether the frame drawn now has a string with `part` in it.
fn shows_part(app: &mut Faraday, part: &str) -> bool {
    app.drawn_texts().iter().any(|t| t.contains(part))
}

fn held(app: &Faraday, fp: [u8; 4]) -> bool {
    app.session.keys.position(Fingerprint(fp)).is_some()
}

/// A Testnet session with no samples: typed as everywhere outside Demo.
fn testnet() -> Faraday {
    let mut app = testkit::started();
    show(&mut app, 1280, 4000, 160);
    app.online = true;
    testkit::move_to(&mut app.session, Network::Testnet).expect("testnet");
    app
}

/// Thirty dice: six words of the long list.
const ROLLS: &str = "123451234512345123451234512345";

// Add a key.

#[test]
fn in_demo_add_a_keys_passphrase_is_chosen_not_typed_and_is_faraday() {
    let mut app = demo_app();
    app.press(Action::Entry(None));
    app.press(Action::UseSample);
    let _ = app.frame();
    assert!(app.entry.passphrase.is_empty());
    assert!(
        shows(&mut app, "No passphrase"),
        "an optional passphrase stays optional"
    );
    app.press(Action::EntryPassphrase);
    assert_eq!(app.entry.passphrase.as_str(), "faraday");
    assert!(shows(&mut app, "faraday"), "drawn unmasked in a code box");
    type_in(&mut app, "x");
    assert_eq!(app.entry.passphrase.as_str(), "faraday", "no typing");
    app.press(Action::EntryAdd);
    let sample = words_of("twelve");
    assert!(
        held(&app, sample.with_passphrase.unwrap()),
        "the sample with faraday is in: {:?}",
        app.entry.error
    );
    assert!(!held(&app, sample.plain.unwrap()));

    // Pressed again, the passphrase is none again.
    app.press(Action::Entry(None));
    app.press(Action::UseSample);
    app.press(Action::EntryPassphrase);
    app.press(Action::EntryPassphrase);
    assert!(app.entry.passphrase.is_empty());
}

#[test]
fn on_testnet_add_a_keys_passphrase_is_typed_masked_and_edited_as_before() {
    let mut app = testnet();
    app.press(Action::Entry(None));
    app.press(Action::EntryPassphrase);
    assert!(app.entry.on_passphrase, "a press focuses the field");
    type_in(&mut app, "ab");
    assert_eq!(app.entry.passphrase.as_str(), "ab");
    app.event(Event::Key(Key::Backspace));
    assert_eq!(app.entry.passphrase.as_str(), "a");
    let _ = app.frame();
    assert!(shows(&mut app, "•"), "masked");
    assert!(!shows(&mut app, "a"));
}

// New key and the typed key's Key card.

#[test]
fn in_demo_a_typed_key_with_a_passphrase_loads_the_key_with_faraday() {
    use faraday_core::keygen::{Start, kstep};
    let mut app = demo_app();
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(Start::Words.index()));
    app.press(Action::KNext);
    let _ = app.frame();
    type_in(&mut app, words_of("twelve").parts[0]);
    let _ = app.frame();
    app.press(Action::KNext);
    assert_eq!(app.keygen.as_ref().unwrap().open, Some(kstep::KEY));
    let _ = app.frame();
    assert!(app.offers(Action::KPassField(0)));
    app.press(Action::KPassField(0));
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.passphrase.as_str(), "faraday");
    assert_eq!(
        k.passphrase2.as_str(),
        "faraday",
        "the repeat is filled too"
    );
    type_in(&mut app, "z");
    assert_eq!(
        app.keygen.as_ref().unwrap().passphrase.as_str(),
        "faraday",
        "no typing"
    );
    let _ = app.frame();
    assert!(
        !shows_part(&mut app, "Never type a seed"),
        "nothing to type"
    );
    app.press(Action::KLock);
    assert!(
        held(&app, words_of("twelve").with_passphrase.unwrap()),
        "{:?}",
        app.keygen.as_ref().unwrap().note
    );
}

// Create a vault, unlock, the sample vault.

#[test]
fn in_demo_create_a_vault_fills_both_fields_with_faraday_and_unlocks_with_it() {
    let mut app = demo_app();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    let c = app.vaults.create.as_ref().expect("the form");
    assert_eq!(c.phrases[0].0.text.as_str(), "faraday");
    assert_eq!(c.phrases[0].1.text.as_str(), "faraday");
    let _ = app.frame();
    assert!(shows(&mut app, "faraday"));
    assert!(!app.offers(Action::Vault(V::CShow)), "no eye");
    assert!(
        !shows_part(&mut app, "Never type a seed"),
        "nothing can be typed there"
    );
    assert!(
        !shows_part(&mut app, "The two passphrases differ"),
        "never shown"
    );
    app.press(Action::Vault(V::CFocus(0, false)));
    type_in(&mut app, "other");
    let c = app.vaults.create.as_ref().expect("the form");
    assert_eq!(c.phrases[0].0.text.as_str(), "faraday", "no typing");
    let _ = app.frame();
    for i in 0..8u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x50 + i; 32],
        )));
    }
    app.press(Action::Vault(V::CGo));
    settle(&mut app);
    assert_eq!(
        app.screen,
        Screen::Unlock,
        "the vault was made: {:?}",
        app.vaults.create.as_ref().map(|c| c.error.clone())
    );
    app.press(Action::Vault(V::Unlock));
    settle(&mut app);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
}

#[test]
fn in_demo_generate_with_dice_shows_the_rolled_words_then_puts_faraday_in_place() {
    let mut app = demo_app();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::Dice(0)));
    type_in(&mut app, ROLLS);
    let words = faraday_core::vault::dice_words(ROLLS, faraday_bip::diceware::List::Large);
    let _ = app.frame();
    assert!(shows_part(&mut app, words[0]), "the rolled words show");
    app.press(Action::Vault(V::DiceUse));
    let _ = app.frame();
    let c = app.vaults.create.as_ref().expect("the form");
    assert_eq!(c.phrases[0].0.text.as_str(), "faraday");
    assert!(shows(&mut app, "Demo puts faraday in place of these words"));
    assert!(
        !shows_part(&mut app, "bits from dice"),
        "no strength line for faraday"
    );
}

#[test]
fn on_testnet_a_vault_takes_a_short_passphrase_typed_two_that_differ_are_refused_and_the_dice_are_measured()
 {
    let mut app = testnet();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_in(&mut app, "ab");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_in(&mut app, "ac");
    let c = app.vaults.create.as_ref().expect("the form");
    assert_eq!(c.phrases[0].0.text.as_str(), "ab", "taken as typed");
    assert_eq!(c.phrases[0].1.text.as_str(), "ac");
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::CShow)), "the eye is there");
    assert!(
        shows(&mut app, "Never type a seed that holds real bitcoin here."),
        "a field a person can type in keeps its caution"
    );
    app.press(Action::Vault(V::CGo));
    settle(&mut app);
    assert_eq!(app.screen, Screen::CreateVault, "two that differ: refused");
    // The dice are measured.
    app.press(Action::Vault(V::Dice(0)));
    type_in(&mut app, ROLLS);
    app.press(Action::Vault(V::DiceUse));
    let _ = app.frame();
    assert!(shows_part(&mut app, "bits from dice"));
}

#[test]
fn in_demo_the_unlock_passphrase_is_faraday_and_takes_no_typing() {
    let mut app = demo_app();
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    assert!(
        app.offers(Action::UseSample),
        "no vault: a sample is offered"
    );
    app.press(Action::UseSample);
    assert_eq!(app.vault_files().len(), 1, "the sample vault is listed");
    assert_eq!(app.vault_files()[0].name, "sample.ofv");
    app.press(Action::Vault(V::Open(0)));
    assert_eq!(app.vaults.passphrase.text.as_str(), "faraday");
    let _ = app.frame();
    assert!(shows(&mut app, "faraday"), "drawn in its code box");
    assert!(!app.offers(Action::Vault(V::ShowTyped)), "no eye");
    type_in(&mut app, "wrong");
    assert_eq!(app.vaults.passphrase.text.as_str(), "faraday");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    assert_eq!(
        app.sheet,
        Some(Sheet::UnlockLoad),
        "it unlocks to the Load sheet"
    );
}

#[test]
fn on_testnet_the_unlock_passphrase_is_typed_and_masked_and_no_sample_is_offered() {
    let mut app = testnet();
    app.samples = Some(&SAMPLES);
    app.press(Action::Nav(Screen::Vaults));
    assert!(!app.offers(Action::UseSample));
    let (name, bytes) = ("v.ofv".to_string(), SAMPLES.inputs.vault.to_vec());
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: vec![(name, bytes)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    assert!(app.vaults.passphrase.text.is_empty(), "nothing filled in");
    app.press(Action::Vault(V::FocusPassphrase));
    type_in(&mut app, "no");
    assert_eq!(app.vaults.passphrase.text.as_str(), "no");
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::ShowTyped)), "the eye");
}

// Lightning.

#[test]
fn in_demo_lightnings_cipher_seed_row_fills_lnds_vector_and_shows_its_node_id() {
    let mut app = demo_app();
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    let _ = app.frame();
    assert!(app.offers(Action::UseSample));
    app.press(Action::UseSample);
    let sample = SAMPLES
        .keys
        .iter()
        .find(|k| k.form == SampleForm::Aezeed)
        .expect("the cipher seed");
    assert_eq!(
        app.lightning.as_ref().unwrap().typed.as_str(),
        sample.parts[0]
    );
    app.press(Action::LResolve);
    settle(&mut app);
    let l = app.lightning.as_ref().unwrap();
    assert_eq!(l.source, Some(NodeSource::Aezeed));
    assert_eq!(
        *l.node.as_ref().expect("a node").public(),
        sample.node.unwrap()
    );
    assert!(
        app.demo_refuses(Action::LShow),
        "the private key stays hidden"
    );
}

#[test]
fn in_demo_a_cipher_seed_that_is_not_the_vector_is_refused() {
    let mut app = demo_app();
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    type_in(&mut app, &["abandon"; 24].join(" "));
    app.press(Action::LResolve);
    settle(&mut app);
    let l = app.lightning.as_ref().unwrap();
    assert!(l.node.is_none());
    assert!(l.error.is_some());
}

#[test]
fn in_demo_an_aezeed_passphrase_is_chosen_not_typed_and_is_faraday() {
    let mut app = demo_app();
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    app.press(Action::LPassphrase);
    assert_eq!(
        app.lightning.as_ref().unwrap().passphrase.as_str(),
        "faraday"
    );
    let _ = app.frame();
    assert!(shows(&mut app, "faraday"));
    type_in(&mut app, "q");
    assert_eq!(
        app.lightning.as_ref().unwrap().passphrase.as_str(),
        "faraday"
    );
}

// Find an address, and a signed message.

/// Spending loaded, through the Load samples sheet.
fn with_spending(app: &mut Faraday) {
    app.press(Action::Samples);
    let at = SAMPLES
        .wallets
        .iter()
        .position(|w| w.name == "sample_spending")
        .expect("Spending");
    app.press(Action::SampleTick(Pick::Wallet(at as u8)));
    app.press(Action::SamplesLoad);
    assert!(!app.session.wallets.is_empty());
}

#[test]
fn in_demo_find_an_addresss_field_takes_the_sample_address_and_finds_it_in_spending() {
    let mut app = demo_app();
    with_spending(&mut app);
    app.press(Action::Explore);
    app.press(Action::X(XAction::Page(Page::Find)));
    let _ = app.frame();
    assert!(app.offers(Action::UseSample));
    app.press(Action::UseSample);
    assert_eq!(app.explore.as_ref().unwrap().query, SAMPLES.inputs.address);
    app.press(Action::X(XAction::Search));
    settle(&mut app);
    let e = app.explore.as_ref().unwrap();
    assert!(e.search.as_ref().is_some_and(|s| s.matched > 0), "found");
}

#[test]
fn in_demo_the_check_a_signed_message_tile_checks_the_sample_message() {
    let mut app = demo_app();
    with_spending(&mut app);
    let at = TILES
        .iter()
        .position(|t| matches!(t.go, Go::CheckMessage))
        .expect("the tile");
    app.press(Action::Catalog(at as u8));
    assert_eq!(app.screen, Screen::CheckMessage);
    let _ = app.frame();
    assert!(shows_part(&mut app, "The signature is valid"));
    // On Testnet, with nothing in the Inbox, the tile opens nothing.
    let mut app = testnet();
    app.samples = Some(&SAMPLES);
    app.press(Action::Catalog(at as u8));
    assert_ne!(app.screen, Screen::CheckMessage);
}

// A backup's passphrase.

#[test]
fn in_demo_the_sample_backup_opens_with_faraday_filled_in() {
    let mut app = demo_app();
    let backup = SAMPLES
        .keys
        .iter()
        .find(|k| k.form == SampleForm::Backup)
        .expect("the backup");
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: vec![("sample.oskb".to_string(), backup.file.to_vec())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::BackupOpen(0));
    assert_eq!(app.sheet, Some(Sheet::Potential));
    assert_eq!(
        app.potential.as_ref().unwrap().passphrase.as_str(),
        "faraday"
    );
    let _ = app.frame();
    assert!(shows(&mut app, "faraday"));
    type_in(&mut app, "x");
    assert_eq!(
        app.potential.as_ref().unwrap().passphrase.as_str(),
        "faraday"
    );
    app.press(Action::PotentialMake);
    settle(&mut app);
    assert!(held(&app, backup.plain.unwrap()), "the backup opened");
}

// An entry's password, a KeePass export, a seed read, a slot.

#[test]
fn in_demo_an_entrys_password_and_the_keepass_exports_are_faraday() {
    use faraday_vault::records::kind;
    let mut app = demo_app();
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    app.press(Action::UseSample);
    app.press(Action::Vault(V::Open(0)));
    app.press(Action::Vault(V::Unlock));
    settle(&mut app);
    if app.sheet == Some(Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadTicked));
    }
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(2)));
    app.press(Action::Vault(V::Add));
    let _ = app.frame();
    let form = app.vaults.form.as_ref().expect("the entry form");
    assert_eq!(form.fields[2].2.text.as_str(), "faraday", "a new password");
    app.press(Action::Vault(V::FocusField(0)));
    type_in(&mut app, "Mail");
    app.press(Action::Vault(V::FocusField(2)));
    type_in(&mut app, "x");
    let form = app.vaults.form.as_ref().expect("the entry form");
    assert_eq!(form.fields[0].2.text.as_str(), "Mail", "the title is typed");
    assert_eq!(
        form.fields[2].2.text.as_str(),
        "faraday",
        "the password is not"
    );
    assert!(shows(&mut app, "faraday"), "drawn in its code box");
    app.press(Action::Vault(V::FormSave));
    let entries = app
        .vaults
        .records(app.vaults.open[0].id)
        .of(kind::ENTRY)
        .count();
    app.press(Action::Vault(V::Item(entries - 1)));
    app.press(Action::Vault(V::ExportKdbx));
    assert_eq!(
        app.vaults
            .prompt
            .as_ref()
            .expect("the prompt")
            .text
            .text
            .as_str(),
        "faraday"
    );
    type_in(&mut app, "x");
    // The export keeps its minimum of 8 characters, which faraday does not
    // meet: it says so and writes nothing.
    app.press(Action::Vault(V::PromptGo));
    settle(&mut app);
    assert_eq!(
        app.vaults.prompt.as_ref().and_then(|p| p.error.as_deref()),
        Some("A passphrase of 8 characters or more")
    );
    assert!(app.outbox.iter().all(|i| !i.name.ends_with(".kdbx")));
}

#[test]
fn in_demo_a_seed_read_adds_its_passphrase_as_faraday_and_loads_that_key() {
    use faraday_core::seeds::read::Choice;
    let mut app = demo_app();
    app.press(Action::Entry(None));
    app.press(Action::ScanSeed);
    let _ = app.frame();
    app.press(Action::ScanSample);
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    app.press(Action::SeedRead(Choice::AddPass));
    let s = app.seed_read.as_ref().expect("the sheet");
    assert_eq!(s.pass.as_str(), "faraday");
    assert_eq!(s.pass2.as_str(), "faraday", "the repeat is filled the same");
    let _ = app.frame();
    assert!(!shows_part(&mut app, "The two passphrases differ"));
    type_in(&mut app, "q");
    assert_eq!(app.seed_read.as_ref().unwrap().pass.as_str(), "faraday");
    app.press(Action::SeedRead(Choice::WithPass));
    let with: Vec<[u8; 4]> = SAMPLES
        .keys
        .iter()
        .filter_map(|k| k.with_passphrase)
        .collect();
    assert!(
        app.session
            .keys
            .all()
            .any(|k| with.contains(&k.fingerprint.0)),
        "the sample with faraday is in: {:?}",
        app.seed_read.as_ref().map(|s| s.error.clone())
    );
}

#[test]
fn in_demo_a_wallet_slots_passphrase_is_faraday() {
    use faraday_core::cstep;
    let mut app = demo_app();
    app.press(Action::Entry(None));
    app.press(Action::UseSample);
    app.press(Action::EntryAdd);
    let first = app.session.keys[0].fingerprint.0;
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CSlotHere(0, first));
    let _ = app.frame();
    app.press(Action::CPassOpen(0));
    let c = app.create.as_ref().expect("Create");
    assert_eq!(c.pass.as_str(), "faraday");
    assert_eq!(c.pass2.as_str(), "faraday");
    let _ = app.frame();
    assert!(shows(&mut app, "faraday"));
    assert!(
        !shows_part(&mut app, "Never type a seed"),
        "nothing can be typed there"
    );
    app.press(Action::CPassLock);
    let with = SAMPLES
        .keys
        .iter()
        .filter_map(|k| k.with_passphrase)
        .collect::<Vec<_>>();
    let c = app.create.as_ref().expect("Create");
    assert!(
        matches!(c.slots[0], faraday_core::create::Source::Here(fp) if with.contains(&fp)),
        "the slot holds the key with faraday: {:?}",
        c.pass_note
    );
}
