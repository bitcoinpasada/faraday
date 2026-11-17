//! A seed Faraday Demo wrote into a vault on an online computer is marked
//! in the vault, and on Mainnet the device asks for a hold before it keeps
//! such a key; a public sample seed is refused there, with no way to keep it.

#![expect(
    clippy::disallowed_methods,
    reason = "a test builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use crate::common::type_text;
use faraday_bip::keys::Network;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_vault::records::{MADE_ONLINE, field, kind};

const PHRASE: &str = "online phrase";

/// The guide's sample key (`docs/MODES.md` §5).
const SAMPLE: &str =
    "brand hobby ranch shoulder brass hockey ranch short brand hockey random torch";

/// Twelve words nobody published and no sample holds.
fn fresh() -> String {
    use faraday_bip::bip39::{Language, Mnemonic};
    let m = Mnemonic::from_entropy(
        Language::English,
        &[
            0x5a, 0x17, 0xc3, 0x09, 0x71, 0xee, 0x2b, 0x84, 0x30, 0xd5, 0x6f, 0x12, 0xa8, 0x4c,
            0x93, 0xfb,
        ],
    )
    .unwrap();
    let words: Vec<String> = m
        .indices()
        .iter()
        .map(|&i| {
            Language::English
                .typed(i)
                .unwrap()
                .as_chars()
                .iter()
                .collect()
        })
        .collect();
    words.join(" ")
}

type Files = Vec<(String, Vec<u8>)>;

fn process(inbox: Files, network: Network) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(inbox),
        outbox: Vec::new(),
        kept: faraday_core::kept_from::<String>(Vec::new()),
    });
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x50 + i; 32])));
    }
    testkit::move_to(&mut app.session, network).unwrap();
    app
}

fn settle(app: &mut Faraday, until: impl Fn(&Faraday) -> bool) {
    for t in 1..60u64 {
        if until(app) {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

fn typed(app: &mut Faraday, words: &str) {
    app.press(Action::Entry(None));
    type_text(app, words);
    app.press(Action::EntryAdd);
}

fn unlock(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults().open.is_empty());
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
    if app.sheet == Some(Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadTicked));
    }
}

/// A device on `network` with a vault holding `records`, unlocked.
fn opened_records(records: Vec<faraday_vault::Record>, network: Network, demo: bool) -> Faraday {
    let file = testkit::kdbx_vault("Made", PHRASE, 7, records).unwrap();
    let mut app = process(vec![("made.kdbx".into(), file)], network);
    if demo {
        testkit::into_demo(&mut app);
    }
    unlock(&mut app);
    app
}

fn key_record(words: &str, flags: Option<u8>) -> faraday_vault::Record {
    let m = faraday_bip::bip39::Mnemonic::parse(
        &testkit::compute(),
        faraday_bip::bip39::Language::English,
        words,
    )
    .unwrap();
    let mut r = faraday_vault::Record::new(kind::KEY)
        .with(field::KEY, &faraday_vault::records::words_payload(&m));
    if let Some(f) = flags {
        r.push(field::KEY_FLAGS, &[f]);
    }
    r
}

/// A Mainnet device with a vault holding `words`, flagged `flags`.
fn opened_with(words: &str, flags: Option<u8>) -> Faraday {
    opened_records(vec![key_record(words, flags)], Network::Mainnet, false)
}

/// The records of `kind` in open vault `o`.
fn records_of(app: &Faraday, o: usize, k: u8) -> Vec<faraday_vault::Record> {
    let id = app.vaults().open[o].id;
    app.vaults()
        .store()
        .contents_for_tests(id)
        .iter()
        .filter(|r| r.kind == k)
        .map(|r| {
            r.fields
                .iter()
                .fold(faraday_vault::Record::new(r.kind), |n, f| {
                    n.with(f.number, &f.bytes)
                })
        })
        .collect()
}

#[test]
fn a_key_demo_wrote_on_an_online_computer_is_kept_on_mainnet_only_by_a_hold() {
    let mut app = opened_with(&fresh(), Some(MADE_ONLINE));
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Published));
    app.press(Action::PublishedKeep);
    assert_eq!(
        app.sheet,
        Some(Sheet::Published),
        "a press does not keep it"
    );
    app.press(Action::PublishedHold);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn a_key_made_on_the_device_brings_no_caution_in_a_vault() {
    let mut app = opened_with(&fresh(), None);
    let _ = app.frame();
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn a_public_sample_seed_in_a_vault_is_refused_on_mainnet() {
    let mut app = opened_with(SAMPLE, None);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Published));
    app.press(Action::PublishedKeep);
    app.press(Action::PublishedHold);
    assert_eq!(app.sheet, Some(Sheet::Published), "it cannot be kept");
    app.press(Action::PublishedRemove);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.keys.len(), 0);
}

#[test]
fn a_key_saved_into_a_vault_by_the_desktop_app_carries_the_online_mark() {
    let file = testkit::kdbx_vault("Made", PHRASE, 8, Vec::new()).unwrap();
    let mut app = process(vec![("made.kdbx".into(), file)], Network::Testnet);
    app.online = true;
    typed(&mut app, &fresh());
    unlock(&mut app);
    app.press(Action::Vault(V::SaveKey(0)));
    let id = app.vaults().open[0].id;
    let flags = app
        .vaults()
        .records(id)
        .of(kind::KEY)
        .find_map(|(_, r)| r.public(field::KEY_FLAGS).and_then(|f| f.first().copied()))
        .expect("the key record has flags");
    assert_eq!(flags & MADE_ONLINE, MADE_ONLINE);
}

/// A desktop app on Testnet with an empty vault open and `words` typed.
fn desktop_with_vault() -> Faraday {
    let file = testkit::kdbx_vault("Made", PHRASE, 8, Vec::new()).unwrap();
    let mut app = process(vec![("made.kdbx".into(), file)], Network::Testnet);
    app.online = true;
    typed(&mut app, &fresh());
    unlock(&mut app);
    app
}

#[test]
fn a_child_seed_saved_into_a_vault_by_the_desktop_app_is_held_on_the_device() {
    let mut app = desktop_with_vault();
    app.press(Action::Bip85);
    app.press(Action::PApp(0));
    for _ in 0..4 {
        app.press(Action::PLength(-1));
    }
    app.press(Action::PNext);
    app.event(Event::Key(faraday_shell_api::Key::Char('7')));
    app.press(Action::PNext);
    app.press(Action::PVault);
    // On the desktop app the child asks whether it is recorded as a child seed.
    app.press(Action::PRecord(true));
    let saved = records_of(&app, 0, kind::KEY);
    assert_eq!(saved.len(), 1, "the child");
    let child = saved
        .iter()
        .find(|r| r.field(field::KEY_CHILD).is_some())
        .expect("the child is in the vault");
    assert!(faraday_vault::records::is_made_online(child));
    let mut device = opened_records(records_of(&app, 0, kind::KEY), Network::Mainnet, false);
    let _ = device.frame();
    assert_eq!(device.sheet, Some(Sheet::Published));
    device.press(Action::PublishedKeep);
    assert_eq!(
        device.sheet,
        Some(Sheet::Published),
        "a press does not keep it"
    );
}

#[test]
fn a_words_file_added_as_a_key_by_the_desktop_app_is_held_on_the_device() {
    let file = testkit::kdbx_vault("Made", PHRASE, 8, Vec::new()).unwrap();
    let words = (fresh() + "\n").into_bytes();
    let mut app = process(
        vec![("made.kdbx".into(), file), ("seed.txt".into(), words)],
        Network::Testnet,
    );
    app.online = true;
    unlock(&mut app);
    app.press(Action::Nav(Screen::Vaults));
    let at = app
        .inbox()
        .iter()
        .position(|i| i.name == "seed.txt")
        .unwrap();
    app.press(Action::Vault(V::AddFile(at)));
    let keys = records_of(&app, 0, kind::KEY);
    assert_eq!(keys.len(), 1, "the file is a key in the vault");
    assert!(faraday_vault::records::is_made_online(&keys[0]));
}

#[test]
fn a_seed_split_into_codex32_strings_by_the_desktop_app_is_held_on_the_device() {
    use crate::codex32_kept::{
        A, C, FP, PHRASE as HOME, codex32_item, key_from_strings, share_record, type_each_back,
        unlock_on_vaults, vault_of, wallet_and_backup,
    };
    use faraday_core::board::Thing;
    // The desktop app keeps string D in its vault beside the paper.
    let mut app = crate::common::device(vec![("home.kdbx".into(), vault_of(HOME, Vec::new(), 7))]);
    app.online = true;
    unlock_on_vaults(&mut app, "home.kdbx", HOME);
    key_from_strings(&mut app);
    wallet_and_backup(&mut app);
    crate::common::vault_at(&mut app, 0, 0, &[]);
    crate::common::into_vault(&mut app, 0, Thing::String(0, crate::codex32_kept::ix('d')));
    codex32_item(&mut app);
    type_each_back(&mut app);
    app.press(Action::BVaultSave(0));
    let shares = records_of(&app, 0, kind::CODEX32_SHARE);
    assert_eq!(shares.len(), 1);
    assert!(faraday_vault::records::is_made_online(&shares[0]));
    // On the device, shares so marked bring their key in held.
    let marked: Vec<_> = [A, C]
        .iter()
        .map(|s| {
            let mut r = share_record(s);
            faraday_vault::records::mark_made_online(&mut r);
            r
        })
        .collect();
    assert_eq!(marked[0].field(field::CODEX32_KEY), Some(&FP[..]));
    let mut device = opened_records(marked, Network::Mainnet, false);
    // The strings load at unlock; the sheet comes up on the next tick.
    device.event(Event::Tick { now_ms: 90_000 });
    let _ = device.frame();
    assert_eq!(device.session.keys.len(), 1);
    assert_eq!(device.sheet, Some(Sheet::Published));
}

#[test]
fn a_child_seed_a_made_online_seed_writes_into_a_vault_is_held_when_opened_on_the_device() {
    let mut app = opened_records(
        vec![key_record(&fresh(), Some(MADE_ONLINE))],
        Network::Mainnet,
        false,
    );
    let _ = app.frame();
    app.press(Action::PublishedHold);
    assert_eq!(app.sheet, None);
    app.press(Action::Bip85);
    app.press(Action::PApp(0));
    for _ in 0..4 {
        app.press(Action::PLength(-1));
    }
    app.press(Action::PNext);
    app.event(Event::Key(faraday_shell_api::Key::Char('7')));
    app.press(Action::PNext);
    app.press(Action::PVault);
    app.press(Action::PRecord(true));
    let child: Vec<_> = records_of(&app, 0, kind::KEY)
        .into_iter()
        .filter(|r| r.field(field::KEY_CHILD).is_some())
        .collect();
    assert_eq!(child.len(), 1, "the child is in the vault");
    assert!(faraday_vault::records::is_made_online(&child[0]));
    let mut device = opened_records(child, Network::Mainnet, false);
    let _ = device.frame();
    assert_eq!(device.sheet, Some(Sheet::Published));
    device.press(Action::PublishedKeep);
    assert_eq!(
        device.sheet,
        Some(Sheet::Published),
        "a press does not keep it"
    );
}

#[test]
fn a_codex32_string_typed_with_made_online_shares_in_a_vault_loads_a_held_key() {
    use crate::codex32_kept::{A, C, share_record};
    let mut c = share_record(C);
    faraday_vault::records::mark_made_online(&mut c);
    let mut app = opened_records(vec![c], Network::Mainnet, false);
    assert!(app.session.keys.is_empty(), "one share is nothing");
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(2));
    type_text(&mut app, A);
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    assert_eq!(app.session.keys.len(), 1);
    // The strings are a published vector, so the sheet shows either way:
    // the key is also marked, and kept so in the next vault it is saved to.
    let fp = app.session.keys[0].fingerprint;
    assert!(app.session.keys.made_online(fp), "the key is not marked");
}

/// A Mainnet device holding a made-online key, held, with a second empty
/// vault open beside the first.
fn two_vaults() -> Faraday {
    let other = testkit::kdbx_vault("Other", "other phrase", 9, Vec::new()).unwrap();
    let made = testkit::kdbx_vault(
        "Made",
        PHRASE,
        7,
        vec![key_record(&fresh(), Some(MADE_ONLINE))],
    )
    .unwrap();
    let mut app = process(
        vec![("made.kdbx".into(), made), ("other.kdbx".into(), other)],
        Network::Mainnet,
    );
    unlock(&mut app);
    let _ = app.frame();
    app.press(Action::PublishedHold);
    assert_eq!(app.sheet, None);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(1)));
    type_text(&mut app, "other phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| a.vaults().open.len() == 2);
    if app.sheet == Some(Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadNotNow));
    }
    assert_eq!(app.vaults().open.len(), 2);
    app
}

#[test]
fn a_key_held_on_the_device_keeps_its_mark_when_saved_to_another_vault() {
    let mut app = two_vaults();
    testkit::vault_current(&mut app, 1);
    app.press(Action::Vault(V::SaveKey(0)));
    let saved = records_of(&app, 1, kind::KEY);
    assert_eq!(saved.len(), 1, "the key is saved");
    assert!(faraday_vault::records::is_made_online(&saved[0]));
}

#[test]
fn a_key_copied_to_another_vault_keeps_its_mark() {
    let mut app = two_vaults();
    testkit::vault_current(&mut app, 0);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(0)));
    app.press(Action::Vault(V::Item(0)));
    app.press(Action::Vault(V::CopyTo));
    app.press(Action::Vault(V::CopyPick(0)));
    let copied = records_of(&app, 1, kind::KEY);
    assert_eq!(copied.len(), 1, "the key is copied");
    assert!(faraday_vault::records::is_made_online(&copied[0]));
}

#[test]
fn a_key_on_testnet_brings_no_caution_whatever_wrote_it() {
    let mut app = opened_records(
        vec![key_record(&fresh(), Some(MADE_ONLINE))],
        Network::Testnet,
        false,
    );
    let _ = app.frame();
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn demo_opening_its_own_vault_brings_no_caution() {
    let mut app = opened_records(
        vec![key_record(&fresh(), Some(MADE_ONLINE))],
        Network::Testnet,
        true,
    );
    let _ = app.frame();
    assert_eq!(app.sheet, None);
}

#[test]
fn a_share_an_online_computer_wrote_marks_a_key_already_loaded_from_other_shares() {
    use crate::codex32_kept::{
        A, C, D, FP, PHRASE as HOME, share_record, unlock_on_vaults, vault_of,
    };
    let mut marked = share_record(D);
    faraday_vault::records::mark_made_online(&mut marked);
    let mut app = crate::common::device(vec![
        (
            "home.kdbx".into(),
            vault_of(HOME, vec![share_record(A), share_record(C)], 3),
        ),
        ("bank.kdbx".into(), vault_of("bank phrase", vec![marked], 4)),
    ]);
    unlock_on_vaults(&mut app, "home.kdbx", HOME);
    let k = app
        .session
        .keys
        .position_of(FP)
        .expect("A and C load the key");
    let fp = app.session.keys.info(k).unwrap().fingerprint;
    assert!(!app.session.keys.made_online(fp));
    unlock_on_vaults(&mut app, "bank.kdbx", "bank phrase");
    assert!(app.session.keys.made_online(fp));
}

#[test]
fn a_child_seed_read_before_its_online_parent_in_a_vault_is_held_too() {
    let mut app = desktop_with_vault();
    app.press(Action::Bip85);
    app.press(Action::PApp(0));
    for _ in 0..4 {
        app.press(Action::PLength(-1));
    }
    app.press(Action::PNext);
    app.event(Event::Key(faraday_shell_api::Key::Char('7')));
    app.press(Action::PNext);
    app.press(Action::PVault);
    app.press(Action::PRecord(true));
    let child = records_of(&app, 0, kind::KEY).remove(0);
    let child = child
        .fields
        .iter()
        .filter(|f| f.number != field::KEY_FLAGS)
        .fold(faraday_vault::Record::new(kind::KEY), |n, f| {
            n.with(f.number, &f.bytes)
        });
    assert!(!faraday_vault::records::is_made_online(&child));
    let mut parent = key_record(&fresh(), None);
    faraday_vault::records::mark_made_online(&mut parent);
    let device = opened_records(vec![child, parent], Network::Mainnet, false);
    assert_eq!(device.session.keys.len(), 2);
    for i in 0..2 {
        let fp = device.session.keys.info(i).unwrap().fingerprint;
        assert!(device.session.keys.made_online(fp), "key {i}");
    }
}

#[test]
fn the_same_words_under_another_passphrase_stay_held_when_the_words_were_written_online() {
    let mut app = opened_with(&fresh(), Some(MADE_ONLINE));
    let c = testkit::compute();
    let a = app
        .session
        .keys
        .add_with_passphrase(&c, 0, "extra", "Again")
        .unwrap();
    assert!(app.session.keys.made_online(a));
    let b = app
        .session
        .keys
        .add_with_suffix(&c, 0, "more", "Vanity")
        .unwrap();
    assert!(app.session.keys.made_online(b));
}
