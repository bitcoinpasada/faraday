//! What Files offers for each kind of file copied in: a text file, a
//! seed's words or a descriptor kept in the open vault with one press,
//! and the shares of a split backup put back together into the wallet
//! from Files itself.

use crate::common::RecordsOf as _;
use faraday_bip::keys::Fingerprint;
use faraday_core::strings::EN;
use faraday_core::testkit;
use faraday_core::testkit::Plain;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};
use faraday_vault::records::{field, kind};

const CODES: &str = "Gmail backup codes\n1234 5678\n2345 6789\n";

fn with_files(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(files),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn unlocked(mut files: Vec<(String, Vec<u8>)>) -> Faraday {
    files.insert(
        0,
        ("vault.kdbx".to_string(), testkit::test_vault().unwrap()),
    );
    let mut app = with_files(files);
    app.press(Action::Vault(V::Open(0)));
    for c in testkit::VAULT_PASSPHRASE.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..10u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
    // Its Load sheet's Not now: the vault stays open on Files (F62).
    app.press(Action::Vault(V::LoadNotNow));
    app
}

fn at(app: &Faraday, name: &str) -> usize {
    app.inbox().iter().position(|i| i.name == name).unwrap()
}

#[test]
fn a_text_file_is_kept_and_offered_to_the_vault_as_a_note() {
    let mut app = unlocked(vec![("gmail-codes.txt".into(), CODES.into())]);
    let k = at(&app, "gmail-codes.txt");
    assert_eq!(app.inbox()[k].kind, FileKind::Text);
    app.press(Action::Nav(Screen::Files));
    // Add to vault is in the row's menu, never a button on the card.
    assert!(!files_offers(&mut app, Action::Vault(V::AddFile(k))));
    let _ = app.frame();
    let dots = Action::File(faraday_core::file_menu::FilePress::Menu(
        faraday_core::file_menu::Row::In(k),
    ));
    app.press(dots);
    let lines: Vec<String> = app
        .files_menu()
        .menu
        .as_ref()
        .expect("its menu")
        .lines
        .iter()
        .map(|(l, _)| l.clone())
        .collect();
    assert!(lines.iter().any(|l| l == "Add to vault"), "{lines:?}");
    app.press(Action::Back);
    app.press(Action::Vault(V::AddFile(k)));
    let v = &app.vaults().open[0];
    let note = app
        .vaults()
        .store()
        .contents_for_tests(v.id)
        .of(kind::NOTE)
        .filter_map(|(_, r)| r.text(field::NOTE))
        .find(|t| t.contains("2345 6789"))
        .expect("no note holds the codes");
    assert!(
        note.starts_with("gmail-codes\n"),
        "the note's title is not the file's name"
    );
    assert!(v.changes > 0, "the vault was not marked changed");
}

#[test]
fn a_seed_words_file_goes_into_the_vault_as_a_key() {
    let (name, bytes) = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n.starts_with("seed-2") && n.ends_with("-words.txt"))
        .unwrap();
    let mut app = unlocked(vec![(name.clone(), bytes)]);
    let keys = app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .of(kind::KEY)
        .count();
    app.press(Action::Vault(V::AddFile(at(&app, &name))));
    assert_eq!(
        app.vaults()
            .store()
            .contents_for_tests(app.vaults().open[0].id)
            .of(kind::KEY)
            .count(),
        keys + 1
    );
}

/// A seed file in Spanish (the published vector) added to an open vault
/// becomes a key record, as one in English does.
#[test]
fn a_spanish_seed_words_file_goes_into_the_vault_as_a_key() {
    let name = "seed-es.txt".to_string();
    let words = "ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco abierto\n";
    let mut app = unlocked(vec![(name.clone(), words.as_bytes().to_vec())]);
    assert_eq!(app.inbox()[at(&app, &name)].kind, FileKind::Words);
    let keys = app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .of(kind::KEY)
        .count();
    app.press(Action::Vault(V::AddFile(at(&app, &name))));
    assert_eq!(
        app.vaults()
            .store()
            .contents_for_tests(app.vaults().open[0].id)
            .of(kind::KEY)
            .count(),
        keys + 1
    );
}

/// A signed message whose body is a seed goes into the vault as a key.
#[test]
fn a_signed_message_whose_body_is_a_seed_goes_into_the_vault_as_a_key() {
    let name = "message.txt".to_string();
    let signed = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\nH9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=\nabandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about\n";
    let mut app = unlocked(vec![(name.clone(), signed.as_bytes().to_vec())]);
    assert_eq!(app.inbox()[at(&app, &name)].kind, FileKind::Words);
    let keys = app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .of(kind::KEY)
        .count();
    app.press(Action::Vault(V::AddFile(at(&app, &name))));
    assert_eq!(
        app.vaults()
            .store()
            .contents_for_tests(app.vaults().open[0].id)
            .of(kind::KEY)
            .count(),
        keys + 1
    );
}

const SEED_24: &str = "limb sadness employ night rocket crime garden timber title eight spy stay company embark secret face fork stable mosquito lock visual bamboo decide velvet";

/// The words of `file` added to the open vault: the word counts of the
/// key records' seeds that came from it.
fn added_key_word_counts(file: &str) -> Vec<usize> {
    added_key_word_counts_of(file, SEED_24)
}

/// Like `added_key_word_counts`, against the seed `seed` instead of `SEED_24`.
fn added_key_word_counts_of(file: &str, seed: &str) -> Vec<usize> {
    let c = testkit::compute();
    let name = "seed.txt".to_string();
    let mut app = unlocked(vec![(name.clone(), file.as_bytes().to_vec())]);
    app.press(Action::Vault(V::AddFile(at(&app, &name))));
    let id = app.vaults().open[0].id;
    [12usize, 18, 24]
        .into_iter()
        .filter(|&n| {
            let words = seed.split(' ').skip(24 - n).collect::<Vec<_>>().join(" ");
            let Ok(m) =
                faraday_keys::words::parse(&c, faraday_bip::bip39::Language::English, &words)
            else {
                return false;
            };
            let payload = faraday_vault::records::words_payload(&m);
            app.vaults()
                .store()
                .contents_for_tests(id)
                .iter()
                .any(|r| r.kind == kind::KEY && r.field(field::KEY) == Some(&payload[..]))
        })
        .collect()
}

/// A 24-word seed written six words to a line goes into the vault as the
/// 24-word key, not as its last 12 words (which also check out).
#[test]
fn a_seed_written_six_words_to_a_line_is_added_whole() {
    let file = "limb sadness employ night rocket crime\ngarden timber title eight spy stay\ncompany embark secret face fork stable\nmosquito lock visual bamboo decide velvet\n";
    assert_eq!(added_key_word_counts(file), vec![24]);
}

/// A title line before the words makes a note holding all the words; it does not make the
/// words after it a signed message's body, so no shorter seed is added.
#[test]
fn a_title_line_before_the_words_adds_no_shorter_seed() {
    let file = "My seed\nlimb sadness employ night rocket crime\ngarden timber title eight spy stay\ncompany embark secret face fork stable\nmosquito lock visual bamboo decide velvet\n";
    let counts = added_key_word_counts(file);
    assert!(!counts.contains(&18), "{counts:?}");
    let mut app = unlocked(vec![("seed.txt".to_string(), file.as_bytes().to_vec())]);
    app.press(Action::Vault(V::AddFile(at(&app, "seed.txt"))));
    let notes: Vec<String> = app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .of(kind::NOTE)
        .filter_map(|(_, r)| r.text(field::NOTE).map(str::to_string))
        .filter(|t| t.contains("limb sadness"))
        .collect();
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].replace('\n', " ").contains(SEED_24), "{notes:?}");
}

/// A bitcoin address as the title line of a seed file does not make the words after it a
/// signed message: no shorter seed is added, and one note holds all 24 words.
#[test]
fn an_address_title_before_the_words_adds_no_shorter_seed() {
    const SEED: &str = "area funny under front blood bargain file awkward sea obtain short ten sun verify cube crumble pet room edge grief vast ugly crucial crater";
    let file = format!(
        "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\n{}\n{}\n{}\n{}\n",
        "area funny under front blood bargain",
        "file awkward sea obtain short ten",
        "sun verify cube crumble pet room",
        "edge grief vast ugly crucial crater"
    );
    assert!(added_key_word_counts_of(&file, SEED).is_empty());
    let mut app = unlocked(vec![("seed.txt".to_string(), file.as_bytes().to_vec())]);
    app.press(Action::Vault(V::AddFile(at(&app, "seed.txt"))));
    let notes: Vec<String> = app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .of(kind::NOTE)
        .filter_map(|(_, r)| r.text(field::NOTE).map(str::to_string))
        .filter(|t| t.contains("area funny"))
        .collect();
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].replace('\n', " ").contains(SEED), "{notes:?}");
}

/// The same address title over a seed written one word per line: the first word is not a
/// signature, so the file is not a message and one note holds all 12 words.
#[test]
fn an_address_title_over_one_word_per_line_adds_a_note_with_all_the_words() {
    const SEED: &str = "army van defense carry jealous true garbage claim echo media make crunch";
    let file = format!(
        "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\n{}\n",
        SEED.replace(' ', "\n")
    );
    let mut app = unlocked(vec![("seed.txt".to_string(), file.as_bytes().to_vec())]);
    app.press(Action::Vault(V::AddFile(at(&app, "seed.txt"))));
    let notes: Vec<String> = app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .of(kind::NOTE)
        .filter_map(|(_, r)| r.text(field::NOTE).map(str::to_string))
        .filter(|t| t.contains("army"))
        .collect();
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].replace('\n', " ").contains(SEED), "{notes:?}");
}

/// Whether Files offers `action` anywhere down the page, moved down by
/// key as a person would, each glide ticked through.
fn files_offers(app: &mut Faraday, action: Action) -> bool {
    app.press(Action::Nav(Screen::Files));
    let mut now = 20_000;
    for _ in 0..20 {
        let _ = app.frame();
        if app.offers(action) {
            return true;
        }
        app.event(Event::Key(Key::Down));
        for _ in 0..12 {
            now += 16;
            app.event(Event::Tick { now_ms: now });
        }
    }
    false
}

/// With every vault locked, a text file's row offers nothing for a vault:
/// not Add to vault, not the vault's Unlock, which stays on the vault's
/// own row (F60). Once the vault is open, Add to vault is in the row's
/// menu, never a button on the card.
#[test]
fn a_text_file_offers_add_to_vault_only_while_a_vault_is_open() {
    let vault = ("vault.kdbx".to_string(), testkit::test_vault().unwrap());
    let codes = ("gmail-codes.txt".to_string(), CODES.as_bytes().to_vec());
    let mut app = with_files(vec![vault, codes.clone()]);
    let k = at(&app, "gmail-codes.txt");
    assert!(!files_offers(&mut app, Action::Vault(V::AddFile(k))));
    assert!(!files_offers(
        &mut app,
        Action::Vault(V::OpenFrom(0, Screen::Files))
    ));
    assert!(
        files_offers(&mut app, Action::Vault(V::Open(0))),
        "the vault's own row unlocks it"
    );
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        !texts
            .iter()
            .any(|t| t == "Add to vault" || t == "Unlock vault.kdbx"),
        "{texts:?}"
    );

    let mut app = unlocked(vec![codes]);
    let k = at(&app, "gmail-codes.txt");
    assert!(!files_offers(&mut app, Action::Vault(V::AddFile(k))));
}

#[test]
fn without_an_open_vault_nothing_is_added() {
    let mut app = with_files(vec![("gmail-codes.txt".into(), CODES.into())]);
    app.press(Action::Vault(V::AddFile(0)));
    assert!(app.vaults().open.is_empty());
    assert!(app.toast_text().is_some_and(|t| t.contains("Unlock")));
}

#[test]
fn files_restores_a_wallet_from_its_shares() {
    let c = faraday_core::testkit::compute();
    let shares: Vec<(String, Vec<u8>)> = testkit::files()
        .unwrap()
        .into_iter()
        .filter(|(n, _)| n.starts_with("savings-share-"))
        .take(2)
        .collect();
    let mut app = with_files(shares);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(
        app.offers(Action::RestoreShares),
        "Files does not offer the restore"
    );
    app.press(Action::RestoreShares);
    // Restore opens on its Description card, the shares ticked, and the
    // wallet is read from them.
    assert_eq!(app.screen, Screen::Restore);
    faraday_core::testkit::confirm_wallets(&mut app);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(
        app.session.address(&c, &app.session.wallets[0], false, 0),
        {
            let mut s = faraday_core::wallet::Session::on(testkit::NET);
            s.add_wallet_confirmed(
                &c,
                "Savings",
                &testkit::savings(),
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
            s.address(&c, &s.wallets[0], false, 0)
        }
    );
}

// ---------------------------------------------------------------------
// §4.1: the two halves of Files are named by
// direction, From the stick and For the stick, wherever a person reads
// them.
// ---------------------------------------------------------------------

use faraday_core::{Screen as S, Sheet, StickInfo};

const SCREENS: [S; 27] = [
    S::Home,
    S::WalletsStart,
    S::Wallets,
    S::Spend,
    S::Files,
    S::Entry,
    S::Backup,
    S::Message,
    S::CheckMessage,
    S::Create,
    S::Restore,
    S::Settings,
    S::Vaults,
    S::CreateVault,
    S::KeyGen,
    S::Bip85,
    S::Silent,
    S::Explore,
    S::Lightning,
    S::Tools,
    S::Unlock,
    S::VaultContents,
    S::SpendBackup,
    S::Vanity,
    S::Decode,
    S::Catalog,
    S::Transfer,
];

/// What `app` draws, each string tagged with where it was drawn.
fn texts_at(app: &mut Faraday, at: &str, all: &mut Vec<(String, String)>) {
    all.extend(
        app.drawn_texts()
            .plain()
            .into_iter()
            .map(|t| (at.to_string(), t)),
    );
}

/// A boot stick holding `files`, every read it asks for answered.
fn boot_stick(app: &mut Faraday, files: &[(String, Vec<u8>)]) {
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("boot"),
        label: faraday_text::Text::of("FARADAY"),
        boot: true,
        files: files
            .iter()
            .map(|(n, b)| (faraday_text::Text::of(n), b.len() as u64))
            .collect(),
        vaults: Vec::new(),
    }]));
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::Read { stick, name } = c
            && let Some((_, bytes)) = files.iter().find(|(n, _)| *n == *name)
        {
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: bytes.clone().into(),
            });
        }
    }
    let _ = app.frame();
}

/// Every screen, and every sheet that names the two halves, at a
/// display of `width` x `height` at `dpi`, with files in both: what is
/// drawn, tagged with where.
fn every_text(width: u16, height: u16, dpi: u16) -> Vec<(String, String)> {
    let kit = |name: &str| {
        testkit::files()
            .unwrap()
            .into_iter()
            .find(|(n, _)| n == name)
            .unwrap()
    };
    let mut app = faraday_core::testkit::new_on_test_net();
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
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            kit("vault.kdbx"),
            kit("spending-wallet.txt"),
            (
                faraday_text::Text::of("gmail-codes.txt").to_string(),
                CODES.into(),
            ),
        ]),
        outbox: faraday_core::files_from(vec![kit("savings-wallet.txt")]),
        kept: Vec::new(),
    });
    let mut all = Vec::new();
    // The boot import sheet, then a stick visit with nothing held.
    boot_stick(&mut app, &[("notes.txt".to_string(), b"a note".to_vec())]);
    if app.import().is_some() {
        assert!(app.import_tile_up(), "the list stands on Files");
        texts_at(&mut app, "boot import list", &mut all);
    }
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("STICK"),
        boot: false,
        files: vec![(faraday_text::Text::of("spend.psbt"), 300)],
        vaults: Vec::new(),
    }]));
    app.sheet = None;
    app.press(Action::Nav(S::Files));
    texts_at(&mut app, "Files with a stick", &mut all);
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.sheet = None;
    // A key loaded, and a wallet: every screen, then the sheets.
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    let w = app
        .inbox()
        .iter()
        .position(|i| i.name == "spending-wallet.txt")
        .unwrap();
    testkit::load_wallet(&mut app, w);
    for screen in SCREENS {
        app.screen = screen;
        texts_at(&mut app, &format!("{screen:?}"), &mut all);
    }
    // Files, scrolled to its foot: For the stick, under all that came in.
    app.screen = S::Files;
    let _ = app.frame();
    app.event(Event::Scroll {
        x: width / 2,
        y: height / 2,
        dy: i16::MAX,
    });
    texts_at(&mut app, "Files scrolled", &mut all);
    for sheet in [
        Sheet::Lock,
        Sheet::LockAsk,
        Sheet::Power,
        Sheet::IdleWarn,
        Sheet::Locked,
        Sheet::WriteOut,
    ] {
        app.sheet = Some(sheet);
        texts_at(&mut app, &format!("{sheet:?}"), &mut all);
    }
    app.sheet = None;
    // The secret sheet: a BIP-85 child seed let out.
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.press(Action::PNext);
    app.press(Action::POut);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    texts_at(&mut app, "secret sheet", &mut all);
    all
}

#[test]
fn no_screen_or_sheet_calls_either_half_inbox_or_outbox() {
    for (w, h, dpi) in [(1366, 768, 160), (480, 640, 160)] {
        let all = every_text(w, h, dpi);
        assert!(all.len() > 500, "{w}x{h}: too little drawn to judge");
        let bad: Vec<&(String, String)> = all
            .iter()
            .filter(|(_, t)| {
                let t = t.to_lowercase();
                t.contains("inbox") || t.contains("outbox")
            })
            .collect();
        assert!(bad.is_empty(), "{w}x{h}: {bad:?}");
        assert!(
            all.iter().any(|(_, t)| t == "In Faraday")
                && all.iter().any(|(_, t)| t == "On the stick"),
            "{w}x{h}: Files does not name its two cards In Faraday and On the stick"
        );
    }
}

/// A second copy of an open vault in Files, under another name, is that
/// vault: its row says it is open and offers its Contents, never another
/// vault's Unlock.
#[test]
fn a_copy_of_an_open_vault_under_another_name_is_that_vault() {
    let b = crate::common::vault("B", "b phrase", 0x31);
    let mut app = crate::common::process(
        vec![
            (
                "a.kdbx".to_string(),
                crate::common::vault("A", "a phrase", 0x30),
            ),
            ("b.kdbx".to_string(), b.clone()),
            ("b-copy.kdbx".to_string(), b),
        ],
        Vec::new(),
        Vec::new(),
    );
    app.press(Action::Vault(V::Open(1)));
    crate::common::unlock(&mut app, "b phrase");
    assert_eq!(app.vaults().open.len(), 1, "B did not open");
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    let texts = app.drawn_texts();
    let contents = texts.iter().filter(|t| t.as_str() == "Contents").count();
    // The open vault's own row, and b.kdbx and its copy.
    assert_eq!(contents, 3, "b.kdbx and its copy are both open: {texts:?}");
    let unlock = texts.iter().filter(|t| t.as_str() == "Unlock").count();
    assert_eq!(unlock, 1, "only a.kdbx is locked: {texts:?}");
}

/// A file read from a stick under a name Files already holds, with other
/// bytes, is kept beside it under a numbered name, and the visit says so;
/// the same bytes again change nothing.
#[test]
fn a_second_file_of_the_same_name_is_kept_under_a_numbered_name() {
    let mut app = with_files(vec![("notes.txt".to_string(), b"first\n".to_vec())]);
    app.storage(StorageEvent::Read {
        stick: faraday_text::Text::of("s"),
        name: faraday_text::Text::of("notes.txt"),
        bytes: b"second\n".to_vec().into(),
    });
    let names: Vec<(&str, &[u8])> = app
        .inbox()
        .iter()
        .map(|i| (i.name.as_str(), i.bytes.as_slice()))
        .collect();
    assert_eq!(
        names,
        vec![
            ("notes.txt", b"first\n".as_slice()),
            ("notes-2.txt", b"second\n".as_slice())
        ]
    );
    assert!(
        app.visit()
            .log
            .iter()
            .any(|(l, _)| l.contains("notes-2.txt")),
        "{:?}",
        app.visit().log
    );
    app.storage(StorageEvent::Read {
        stick: faraday_text::Text::of("s"),
        name: faraday_text::Text::of("notes.txt"),
        bytes: b"first\n".to_vec().into(),
    });
    assert_eq!(app.inbox().len(), 2, "the same bytes again are no new file");
}

/// A stick's FAT does not tell names apart by case: a file read under a
/// name that differs from one held only by case is kept numbered.
#[test]
fn a_second_file_whose_name_differs_only_by_case_is_kept_numbered() {
    let mut app = with_files(vec![("Notes.txt".to_string(), b"first\n".to_vec())]);
    app.storage(StorageEvent::Read {
        stick: faraday_text::Text::of("s"),
        name: faraday_text::Text::of("notes.txt"),
        bytes: b"second\n".to_vec().into(),
    });
    let names: Vec<&str> = app.inbox().iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, vec!["Notes.txt", "notes-2.txt"]);
}

/// A From the stick vault row whose name a For the stick vault also
/// has, another vault, is its own vault: with the For the stick one open
/// and its own locked, the row offers Unlock, not the other's Contents.
#[test]
fn a_vault_row_names_its_own_vault_not_another_of_its_name() {
    let mut app = crate::common::process(
        vec![(
            "v.kdbx".to_string(),
            crate::common::vault("B", "b phrase", 0x31),
        )],
        vec![(
            "v.kdbx".to_string(),
            crate::common::vault("A", "a phrase", 0x30),
        )],
        Vec::new(),
    );
    app.press(Action::Vault(V::Open(0)));
    crate::common::unlock(&mut app, "a phrase");
    assert_eq!(app.vaults().open.len(), 1, "A did not open");
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Unlock"), "{texts:?}");
    // Contents is the open vault A's own row in In Faraday; the From the
    // stick row of the other vault of that name offers Unlock only.
    let contents = texts.iter().filter(|t| t.as_str() == "Contents").count();
    assert_eq!(contents, 1, "{texts:?}");
}

/// A vault written to a stick stays in Files as the stick holds it; a
/// file of its name Files holds, another vault, stays too, the written
/// one numbered beside it, and the visit says so.
#[test]
fn a_written_vault_beside_another_of_its_name_is_kept_numbered() {
    let a = crate::common::vault("A", "a phrase", 0x30);
    let b = crate::common::vault("B", "b phrase", 0x31);
    let mut app = crate::common::process(
        vec![("v.kdbx".to_string(), b.clone())],
        vec![("v.kdbx".to_string(), a.clone())],
        Vec::new(),
    );
    app.storage(StorageEvent::Written {
        stick: faraday_text::Text::of("s"),
        name: faraday_text::Text::of("v.kdbx"),
        wrote_as: faraday_text::Text::of("v.kdbx"),
    });
    let held: Vec<(&str, bool)> = app
        .inbox()
        .iter()
        .map(|i| (i.name.as_str(), *i.bytes == a))
        .collect();
    assert_eq!(held, vec![("v.kdbx", false), ("v-2.kdbx", true)]);
    assert!(
        app.visit().log.iter().any(|(l, _)| l.contains("v-2.kdbx")),
        "{:?}",
        app.visit().log
    );
}

/// An Inbox vault file whose vault is open reads "Vault, open", and its
/// own row in In Faraday, with no wallet left to load, has no ⋮ that
/// would open nothing.
#[test]
fn an_open_vaults_inbox_file_reads_open_and_its_row_has_no_empty_menu() {
    let mut app = crate::common::process(
        vec![(
            "a.kdbx".to_string(),
            crate::common::vault("A", "a phrase", 0x30),
        )],
        Vec::new(),
        Vec::new(),
    );
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.starts_with("Vault, locked")),
        "{texts:?}"
    );
    app.press(Action::Vault(V::Open(0)));
    crate::common::unlock(&mut app, "a phrase");
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.starts_with("Vault, open")),
        "{texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.starts_with("Vault, locked")),
        "{texts:?}"
    );
    let menu = faraday_core::file_menu::FilePress::Menu(faraday_core::file_menu::Row::Vault(0));
    assert!(!app.offers(Action::File(menu)), "nothing to load: no ⋮");
}

/// An Outbox file says its state, and an open vault's file on the list
/// says Lock writes it again, with how many changes are not yet saved.
#[test]
fn the_outbox_says_ready_unprotected_and_written_after_lock_with_the_unsaved_changes() {
    let v = crate::common::vault("A", "a phrase", 0x30);
    let mut app = crate::common::process(
        vec![("a.kdbx".to_string(), v.clone())],
        vec![
            ("a.kdbx".to_string(), v),
            ("notes.pdf".to_string(), b"%PDF".to_vec()),
        ],
        Vec::new(),
    );
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Ready"), "{texts:?}");
    assert!(
        !texts.iter().any(|t| t == "Written after Lock"),
        "{texts:?}"
    );
    app.press(Action::Vault(V::Open(0)));
    crate::common::unlock(&mut app, "a phrase");
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Written after Lock"), "{texts:?}");
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("Vault, open · sealed and locked")),
        "{texts:?}"
    );
    // A change not yet saved is counted on the vault's row and the file's.
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::MemoryAsk));
    let _ = app.frame();
    let have = app.vaults().open[0].facts().header.cost.memory_kib;
    let pick = if have == faraday_vault::file::STEPS_KIB[1] {
        2
    } else {
        1
    };
    app.press(Action::Vault(V::MemoryStep(pick)));
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.contains("unsaved change")),
        "{texts:?}"
    );
}

/// Sparrow's descriptor file, as some other tool might have written it: a
/// comment, CRLF lines, `'` for hardened steps and no checksum. With it,
/// the descriptor Faraday reads from it.
fn respelled_descriptor() -> (String, String) {
    let c = testkit::compute();
    let read = faraday_core::wallet::read_wallet(
        &c,
        include_str!("../vectors/interop/sparrow/descriptor-multipath.txt"),
    )
    .unwrap()
    .to_descriptor_checksummed()
    .to_string();
    let body = read.split('#').next().unwrap();
    let other = body.replace("h/", "'/").replace("h]", "']");
    assert_ne!(other, body, "nothing to respell");
    (format!("# kept for the shop\r\n{other}\r\n"), read)
}

/// An empty vault, open, with `name` in Files.
fn empty_vault_with(name: &str, bytes: &[u8]) -> Faraday {
    let vault = testkit::kdbx_vault("Test vault", testkit::VAULT_PASSPHRASE, 0x7a, Vec::new());
    let mut app = with_files(vec![
        ("vault.kdbx".to_string(), vault.unwrap()),
        (name.to_string(), bytes.to_vec()),
    ]);
    open_first_vault(&mut app);
    app
}

fn open_first_vault(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    crate::common::type_text(app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::Vault(V::LoadNotNow));
}

/// The text of each wallet item in the open vault.
fn wallet_items(app: &Faraday) -> Vec<String> {
    app.vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .of(kind::WALLET)
        .map(|(_, r)| r.text(field::WALLET).unwrap_or_default().to_string())
        .collect()
}

/// The vault sealed, the device powered off, and the vault opened again
/// by a new process holding only the written file.
fn sealed_and_reopened(mut app: Faraday) -> Faraday {
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let mut written = Vec::new();
    while let Some(cmd) = app.poll_storage() {
        if let faraday_core::StorageCommand::SaveBoxes { outbox, .. } = cmd {
            written = testkit::plain_files(outbox);
        }
    }
    assert_eq!(written.len(), 1, "the vault file was sealed");
    let mut app = with_files(written);
    open_first_vault(&mut app);
    app
}

/// `bytes` added to a vault from Files, the confirmation sheet held where
/// it asks: the item holds `want`, before power off and after, and Load
/// gives that wallet with nothing asked.
fn goes_in_as(name: &str, bytes: &[u8], want: &str) {
    let mut app = empty_vault_with(name, bytes);
    assert_eq!(app.inbox()[at(&app, name)].kind, FileKind::Wallet);
    app.press(Action::Vault(V::AddFile(at(&app, name))));
    if app.sheet == Some(Sheet::WalletConfirm) {
        app.press(Action::WalletHold);
    }
    assert_eq!(wallet_items(&app), [want], "{:?}", app.toast_text());
    let mut app = sealed_and_reopened(app);
    assert_eq!(wallet_items(&app), [want], "after power off");
    app.press(Action::Vault(V::LoadAll(0)));
    assert_eq!(app.sheet, None, "{:?}", app.toast_text());
    let loaded: Vec<String> = app
        .session
        .wallets
        .iter()
        .map(|w| w.policy().to_descriptor_checksummed().to_string())
        .collect();
    assert_eq!(loaded, [want], "{:?}", app.toast_text());
}

#[test]
fn a_descriptor_file_spelled_another_way_goes_into_the_vault_as_faraday_read_it() {
    let (file, read) = respelled_descriptor();
    goes_in_as("shop.txt", file.as_bytes(), &read);
}

#[test]
fn another_wallets_file_goes_into_the_vault_as_its_descriptor() {
    let file = include_str!("../vectors/interop/sparrow/electrum-multisig-wallet.json");
    let c = testkit::compute();
    let read = faraday_core::wallet::read_wallet(&c, file)
        .unwrap()
        .to_descriptor_checksummed();
    goes_in_as("electrum.json", file.as_bytes(), &read);
}

/// Coldcard's export of one key's single-sig accounts and its silent
/// payments account: Add to vault asks which, and keeps the one chosen.
#[test]
fn add_to_vault_asks_which_account_of_a_coldcard_export_and_keeps_that_one() {
    use faraday_core::wallet::Account;
    let file = include_str!("../vectors/interop/sparrow/cc-singlesig-sp-keystore-1.json");
    let c = testkit::compute();
    let single = faraday_core::wallet::read_wallet(&c, file)
        .unwrap()
        .to_descriptor_checksummed();
    let asked = || {
        let mut app = empty_vault_with("cc.json", file.as_bytes());
        app.press(Action::Vault(V::AddFile(at(&app, "cc.json"))));
        assert_eq!(app.sheet, Some(Sheet::Account), "{:?}", app.toast_text());
        assert!(wallet_items(&app).is_empty(), "kept before a choice");
        app
    };

    let mut app = asked();
    app.press(Action::LoadAccount(None));
    assert!(wallet_items(&app).is_empty(), "kept after Cancel");

    let mut app = asked();
    app.press(Action::LoadAccount(Some(Account::SingleSig)));
    assert_eq!(wallet_items(&app), [single.as_str()]);
    assert!(app.session.wallets.is_empty(), "loaded, not only kept");
    let mut app = sealed_and_reopened(app);
    assert_eq!(wallet_items(&app), [single.as_str()], "after power off");
    app.press(Action::Vault(V::LoadAll(0)));
    let loaded = app.session.wallets[0].policy().to_descriptor_checksummed();
    assert_eq!(loaded.to_string(), single.as_str());

    let mut app = asked();
    app.press(Action::LoadAccount(Some(Account::Silent)));
    let kept = wallet_items(&app);
    assert_eq!(kept.len(), 1, "{:?}", app.toast_text());
    assert!(app.session.wallets.is_empty(), "loaded, not only kept");
    let record = faraday_bip::silent_wallet::SilentWallet::parse(&kept[0]).expect("a record");
    assert_eq!(record.to_text(), kept[0]);
    assert_eq!(record.fingerprint.to_string(), "0f056943");
    let mut app = sealed_and_reopened(app);
    assert_eq!(wallet_items(&app), kept, "after power off");
    app.press(Action::Vault(V::LoadAll(0)));
    assert_eq!(app.sheet, None, "{:?}", app.toast_text());
    let loaded = app.session.wallets[0].policy().silent().expect("silent");
    assert_eq!(loaded.to_text(), kept[0]);
}

const CC_EXPORT: &str = include_str!("../vectors/interop/sparrow/cc-singlesig-sp-keystore-1.json");

/// Add to vault pressed on the export, its account dialog open.
fn asked_which_account() -> Faraday {
    let mut app = empty_vault_with("cc.json", CC_EXPORT.as_bytes());
    app.press(Action::Vault(V::AddFile(at(&app, "cc.json"))));
    assert_eq!(app.sheet, Some(Sheet::Account), "{:?}", app.toast_text());
    app
}

/// `name` read from a stick into Files.
fn read_in(app: &mut Faraday, name: &str, bytes: &[u8]) {
    app.storage(StorageEvent::Read {
        stick: faraday_text::Text::of("s"),
        name: faraday_text::Text::of(name),
        bytes: bytes.to_vec().into(),
    });
}

/// Files holding only `name`, with `bytes`, as a kept list brought back
/// would. A stick read keeps no secret the person did not tick, so a file
/// with one comes in this way.
fn listed(app: &mut Faraday, name: &str, bytes: &[u8]) {
    let name = faraday_text::Text::of(name);
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(name, bytes.to_vec())]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
}

/// The silent payments account chosen in the open dialog went into the
/// vault, once, and no wallet was loaded.
fn chooses_silent_into_the_vault(app: &mut Faraday) {
    app.press(Action::LoadAccount(Some(
        faraday_core::wallet::Account::Silent,
    )));
    let kept = wallet_items(app);
    assert_eq!(kept.len(), 1, "{:?}", app.toast_text());
    faraday_bip::silent_wallet::SilentWallet::parse(&kept[0]).expect("a record");
    assert!(app.session.wallets.is_empty(), "loaded, not kept");
}

#[test]
fn a_file_arriving_under_the_account_dialog_leaves_add_to_vault_adding() {
    let mut app = asked_which_account();
    read_in(&mut app, "later.txt", CODES.as_bytes());
    chooses_silent_into_the_vault(&mut app);
}

#[test]
fn add_to_vault_finds_its_file_when_files_moved_it() {
    let mut app = asked_which_account();
    let before = at(&app, "cc.json");
    read_in(&mut app, "later.txt", CODES.as_bytes());
    let later = app.inbox()[at(&app, "later.txt")].duplicate();
    testkit::inbox_retain(&mut app, |i| i.name.as_str() != "later.txt");
    testkit::inbox_insert(&mut app, 0, later);
    assert_ne!(at(&app, "cc.json"), before);
    chooses_silent_into_the_vault(&mut app);
}

/// The asked-for file gone, or another file now under its name (here the
/// export's own single-sig descriptor, an export of the same silent account
/// with another single-sig one, and the export with seed words beside it):
/// said, and nothing kept or loaded.
#[test]
fn add_to_vault_says_when_the_file_it_asked_about_is_gone() {
    use faraday_core::wallet::Account;
    let c = testkit::compute();
    let single = faraday_core::wallet::read_wallet(&c, CC_EXPORT)
        .unwrap()
        .to_descriptor_checksummed()
        .to_string();
    let mut another: serde_json::Value = serde_json::from_str(CC_EXPORT).unwrap();
    assert!(another.as_object_mut().unwrap().remove("bip84").is_some());
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    let beside = CC_EXPORT.replacen('{', &format!(r#"{{"note":"{}","#, &*words), 1);
    for other in [None, Some(single), Some(another.to_string()), Some(beside)] {
        let mut app = asked_which_account();
        testkit::inbox_retain(&mut app, |i| i.name.as_str() != "cc.json");
        if let Some(other) = &other {
            listed(&mut app, "cc.json", other.as_bytes());
            at(&app, "cc.json");
        }
        app.press(Action::LoadAccount(Some(Account::Silent)));
        assert!(wallet_items(&app).is_empty(), "{other:?}");
        assert!(app.session.wallets.is_empty(), "{other:?}");
        assert_eq!(app.sheet, None, "{other:?}");
        assert_eq!(
            app.toast_text(),
            Some("cc.json is no longer in Files."),
            "{other:?}"
        );
    }
}

/// The name a wallet file carries names the vault's wallet, as it names
/// the one Files' Load gives.
#[test]
fn the_vaults_wallet_takes_the_name_its_file_carries() {
    let (_, read) = respelled_descriptor();
    let file = format!(r#"{{"label":"Shop","blockheight":0,"descriptor":"{read}"}}"#);
    let mut app = empty_vault_with("export.json", file.as_bytes());
    app.press(Action::Vault(V::AddFile(at(&app, "export.json"))));
    assert_eq!(wallet_items(&app).len(), 1, "{:?}", app.toast_text());
    let mut app = sealed_and_reopened(app);
    app.press(Action::Vault(V::LoadAll(0)));
    assert_eq!(app.session.wallets[0].name.as_str(), "Shop");
}

/// An account chosen for a load does not answer Add to vault, which asks
/// each time, and what Add to vault is told does not change what loads.
#[test]
fn add_to_vault_asks_each_time_and_leaves_the_loads_account_as_it_was() {
    use faraday_core::wallet::Account;
    let mut app = empty_vault_with("cc.json", CC_EXPORT.as_bytes());
    let load = |app: &mut Faraday| app.press(Action::LoadWallet(at(app, "cc.json")));
    let silent_loaded = |app: &Faraday| {
        assert_eq!(app.sheet, None, "{:?}", app.toast_text());
        assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
        assert!(app.session.wallets[0].policy().silent().is_some());
    };
    app.press(Action::Vault(V::AddFile(at(&app, "cc.json"))));
    app.press(Action::LoadAccount(Some(Account::SingleSig)));
    assert_eq!(wallet_items(&app).len(), 1, "{:?}", app.toast_text());
    load(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Account), "{:?}", app.toast_text());
    app.press(Action::LoadAccount(Some(Account::Silent)));
    silent_loaded(&app);

    for (chosen, items) in [(Account::SingleSig, 2), (Account::Silent, 3)] {
        app.press(Action::Vault(V::AddFile(at(&app, "cc.json"))));
        assert_eq!(app.sheet, Some(Sheet::Account), "{:?}", app.toast_text());
        assert_eq!(wallet_items(&app).len(), items - 1);
        app.press(Action::LoadAccount(Some(chosen)));
        let kept = wallet_items(&app);
        assert_eq!(kept.len(), items, "{:?}", app.toast_text());
        let silent = faraday_bip::silent_wallet::SilentWallet::parse(&kept[items - 1]);
        assert_eq!(silent.is_some(), chosen == Account::Silent, "{kept:?}");

        app.session.wallets.clear();
        load(&mut app);
        silent_loaded(&app);
    }
}

/// A 1-of-2 over two keys no loaded key is, and keys of its own choosing:
/// the descriptor, checksummed.
fn planted_descriptor() -> String {
    let p = "m/48'/1'/0'/2'";
    let (a, b) = (testkit::key(1, p), testkit::key(2, p));
    checksummed(&format!("wsh(sortedmulti(1,{a}/<0;1>/*,{b}/<0;1>/*))"))
}

/// `d` as Faraday writes the wallet it reads there.
fn checksummed(d: &str) -> String {
    faraday_core::wallet::read_wallet(&testkit::compute(), d)
        .unwrap()
        .to_descriptor_checksummed()
        .to_string()
}

/// Test seed `n` typed in and added to the session's keys.
fn loads_seed(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    crate::common::type_text(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
    app.press(Action::EntryAdd);
}

/// Load pressed on `name` asks the confirmation sheet and loads nothing.
fn load_asks(app: &mut Faraday, name: &str) {
    app.press(Action::LoadWallet(at(app, name)));
    assert_eq!(
        app.sheet,
        Some(Sheet::WalletConfirm),
        "Load of {name}: {:?}",
        app.toast_text()
    );
    assert!(app.session.wallets.is_empty(), "{name}: loaded");
}

/// A planted 1-of-2 in each form a file brings one in.
fn planted_files() -> [(&'static str, String); 4] {
    let d = planted_descriptor();
    // Two of the cosigners of Sparrow's published Electrum wallet.
    let all: serde_json::Value = serde_json::from_str(include_str!(
        "../vectors/interop/sparrow/electrum-multisig-wallet.json"
    ))
    .unwrap();
    let electrum = serde_json::json!({
        "x1/": all["x1/"], "x2/": all["x2/"],
        "wallet_type": "1of2", "use_encryption": false, "seed_version": 17,
    });
    let p = "m/48'/1'/0'/2'";
    let line = |n| {
        let key = testkit::key(n, p);
        let (origin, xpub) = key.split_once(']').unwrap();
        format!("{}: {xpub}\n", origin[1..9].to_uppercase())
    };
    let config = format!(
        "Name: Planted\nPolicy: 1 of 2\nDerivation: {p}\nFormat: P2WSH\n\n{}{}",
        line(1),
        line(2)
    );
    [
        ("electrum.json", electrum.to_string()),
        (
            "wallet.json",
            format!(r#"{{"label":"Shop","blockheight":0,"descriptor":"{d}"}}"#),
        ),
        ("config.txt", config),
        ("planted.txt", d),
    ]
}

/// The places each wallet item of open vault `v` records as checked.
fn item_ticks(app: &Faraday, v: usize) -> Vec<Vec<usize>> {
    app.vaults()
        .store()
        .contents_for_tests(app.vaults().open[v].id)
        .of(kind::WALLET)
        .map(|(_, r)| faraday_vault::records::keys_checked(r))
        .collect()
}

fn add(app: &mut Faraday, name: &str) {
    app.press(Action::Vault(V::AddFile(at(app, name))));
}

/// Add to vault pressed on `name`, its confirmation sheet open and
/// nothing kept or loaded yet.
fn asked_before_keeping(app: &mut Faraday, name: &str) {
    add(app, name);
    assert_eq!(
        app.sheet,
        Some(Sheet::WalletConfirm),
        "{name}: {:?}",
        app.toast_text()
    );
    nothing_kept(app, name);
}

fn nothing_kept(app: &Faraday, name: &str) {
    assert!(wallet_items(app).is_empty(), "{name}: kept");
    assert!(app.session.wallets.is_empty(), "{name}: loaded");
}

/// The text Save to vault writes for the wallet of `name` once Load's
/// sheet is held.
fn as_save_to_vault_writes(name: &str, bytes: &[u8]) -> Vec<String> {
    let mut app = empty_vault_with(name, bytes);
    app.press(Action::LoadWallet(at(&app, name)));
    app.press(Action::WalletHold);
    app.press(Action::Vault(V::SaveWallet(0)));
    wallet_items(&app)
}

#[test]
fn add_to_vault_asks_before_the_vault_keeps_a_wallet_of_several_keys() {
    for (name, file) in planted_files() {
        let saved = as_save_to_vault_writes(name, file.as_bytes());
        assert_eq!(saved.len(), 1, "{name}");
        let mut app = empty_vault_with(name, file.as_bytes());
        asked_before_keeping(&mut app, name);
        app.press(Action::WalletHold);
        assert_eq!(wallet_items(&app), saved, "{name}: {:?}", app.toast_text());
        assert!(app.session.wallets.is_empty(), "{name}: loaded");
        assert_eq!(app.sheet, None, "{name}");
        let mut app = sealed_and_reopened(app);
        assert_eq!(wallet_items(&app), saved, "{name}: after power off");
        app.press(Action::Vault(V::LoadAll(0)));
        assert_eq!(app.sheet, None, "{name}: {:?}", app.toast_text());
        assert_eq!(app.session.wallets.len(), 1, "{name}");
    }
}

#[test]
fn cancel_at_add_to_vaults_sheet_keeps_nothing_and_the_next_press_asks_again() {
    let d = planted_descriptor();
    let mut app = empty_vault_with("planted.txt", d.as_bytes());
    asked_before_keeping(&mut app, "planted.txt");
    app.press(Action::WalletCancel);
    nothing_kept(&app, "planted.txt");
    assert_eq!(app.sheet, None);
    asked_before_keeping(&mut app, "planted.txt");
}

#[test]
fn the_ticks_of_add_to_vaults_sheet_are_the_vaults() {
    let d = planted_descriptor();
    let mut app = empty_vault_with("planted.txt", d.as_bytes());
    asked_before_keeping(&mut app, "planted.txt");
    app.press(Action::KeyTick(faraday_core::wallet::KeyBlockOf::Asked, 1));
    app.press(Action::WalletHold);
    assert_eq!(item_ticks(&app, 0), [vec![1]]);
    let mut app = sealed_and_reopened(app);
    app.press(Action::Vault(V::LoadAll(0)));
    assert_eq!(app.session.wallets[0].key_ticks(), [false, true]);
}

#[test]
fn add_to_vault_asks_nothing_for_a_wallet_of_one_key() {
    let key = testkit::key(1, "m/84'/1'/0'");
    let d = format!("wpkh({key}/<0;1>/*)");
    let mut app = empty_vault_with("one.txt", d.as_bytes());
    add(&mut app, "one.txt");
    assert_eq!(app.sheet, None, "{:?}", app.toast_text());
    assert_eq!(wallet_items(&app).len(), 1);
}

#[test]
fn add_to_vault_asks_nothing_for_a_wallet_confirmed_already() {
    let d = planted_descriptor();
    let mut app = empty_vault_with("planted.txt", d.as_bytes());
    asked_before_keeping(&mut app, "planted.txt");
    app.press(Action::WalletHold);
    add(&mut app, "planted.txt");
    assert_eq!(app.sheet, None, "{:?}", app.toast_text());
    assert_eq!(wallet_items(&app).len(), 2);
}

/// A wallet the session holds without a sheet, here from a vault's own
/// record, is asked about no more.
#[test]
fn add_to_vault_asks_nothing_for_a_wallet_the_session_holds() {
    let d = planted_descriptor();
    let mut app = empty_vault_with("planted.txt", d.as_bytes());
    asked_before_keeping(&mut app, "planted.txt");
    app.press(Action::WalletHold);
    let mut app = sealed_and_reopened(app);
    app.press(Action::Vault(V::LoadAll(0)));
    assert_eq!(app.session.wallets.len(), 1);
    read_in(&mut app, "again.txt", d.as_bytes());
    add(&mut app, "again.txt");
    assert_eq!(app.sheet, None, "{:?}", app.toast_text());
    assert_eq!(wallet_items(&app).len(), 2);
}

#[test]
fn add_to_vault_asks_nothing_when_every_key_is_a_loaded_ones() {
    let d = testkit::savings();
    let mut app = empty_vault_with("ours.txt", d.as_bytes());
    for (seed, _) in &testkit::TEST_SEEDS[..3] {
        app.press(Action::Entry(None));
        crate::common::type_text(&mut app, &testkit::test_words(seed));
        app.press(Action::EntryAdd);
    }
    add(&mut app, "ours.txt");
    assert_eq!(app.sheet, None, "{:?}", app.toast_text());
    assert_eq!(wallet_items(&app).len(), 1);
}

#[test]
fn the_hold_at_add_to_vault_takes_its_file_wherever_files_now_has_it() {
    let d = planted_descriptor();
    let mut app = empty_vault_with("planted.txt", d.as_bytes());
    asked_before_keeping(&mut app, "planted.txt");
    let before = at(&app, "planted.txt");
    read_in(&mut app, "later.txt", CODES.as_bytes());
    let later = app.inbox()[at(&app, "later.txt")].duplicate();
    testkit::inbox_retain(&mut app, |i| i.name.as_str() != "later.txt");
    testkit::inbox_insert(&mut app, 0, later);
    assert_ne!(at(&app, "planted.txt"), before);
    app.press(Action::WalletHold);
    assert_eq!(wallet_items(&app), [d.as_str()], "{:?}", app.toast_text());
    assert!(app.session.wallets.is_empty(), "loaded");
}

/// The file gone; or another wallet, the same keys written in another
/// order, or the same wallet with seed words beside it now under its name.
/// Nothing was kept, so a load of the wallet still asks.
#[test]
fn the_hold_at_add_to_vault_keeps_nothing_once_its_file_is_gone() {
    let d = planted_descriptor();
    let p = "m/48'/1'/0'/2'";
    let (a, b) = (testkit::key(1, p), testkit::key(2, p));
    let reordered = checksummed(&format!("wsh(sortedmulti(1,{b}/<0;1>/*,{a}/<0;1>/*))"));
    assert_ne!(reordered, d);
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    let beside = format!("{d}\n# {}\n", &*words);
    for other in [
        None,
        Some(testkit::savings()),
        Some(reordered),
        Some(beside),
    ] {
        let mut app = empty_vault_with("planted.txt", d.as_bytes());
        asked_before_keeping(&mut app, "planted.txt");
        testkit::inbox_retain(&mut app, |i| i.name.as_str() != "planted.txt");
        if let Some(other) = &other {
            listed(&mut app, "planted.txt", other.as_bytes());
            at(&app, "planted.txt");
        }
        app.press(Action::WalletHold);
        nothing_kept(&app, "planted.txt");
        assert_eq!(
            app.toast_text(),
            Some("planted.txt is no longer in Files."),
            "{other:?}"
        );
        testkit::inbox_retain(&mut app, |i| i.name.as_str() != "planted.txt");
        read_in(&mut app, "planted.txt", d.as_bytes());
        load_asks(&mut app, "planted.txt");
    }
}

#[test]
fn the_hold_at_add_to_vault_never_puts_the_wallet_in_another_vault() {
    let d = planted_descriptor();
    let second = testkit::kdbx_vault("Second", testkit::VAULT_PASSPHRASE, 0x7b, Vec::new());
    let mut app = empty_vault_with("planted.txt", d.as_bytes());
    read_in(&mut app, "second.kdbx", &second.unwrap());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(1)));
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    crate::common::type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 100..160u64 {
        if app.vaults().open.len() == 2 {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 2, "{:?}", app.toast_text());
    app.press(Action::Vault(V::LoadNotNow));
    app.press(Action::Vault(V::Show(0)));
    let first = app.vaults().current_id();
    add(&mut app, "planted.txt");
    assert_eq!(app.sheet, Some(Sheet::WalletConfirm));
    app.press(Action::Vault(V::Show(1)));
    assert_ne!(app.vaults().current_id(), first);
    app.press(Action::WalletHold);
    assert_eq!(item_ticks(&app, 0).len(), 0, "the first vault");
    assert_eq!(item_ticks(&app, 1).len(), 0, "the other vault");
    assert!(app.session.wallets.is_empty(), "loaded");
    assert_eq!(
        app.toast_text(),
        Some("planted.txt was not added: its vault is no longer the one shown.")
    );
    load_asks(&mut app, "planted.txt");
}

/// The same wallet under the same file name, now labelled otherwise: the
/// vault gets no name the sheet did not show.
#[test]
fn the_hold_at_add_to_vault_keeps_nothing_under_a_name_the_sheet_did_not_show() {
    let d = planted_descriptor();
    let file = |label| format!(r#"{{"label":"{label}","blockheight":0,"descriptor":"{d}"}}"#);
    let mut app = empty_vault_with("wallet.json", file("Shop").as_bytes());
    asked_before_keeping(&mut app, "wallet.json");
    testkit::inbox_retain(&mut app, |i| i.name.as_str() != "wallet.json");
    read_in(&mut app, "wallet.json", file("Till").as_bytes());
    at(&app, "wallet.json");
    app.press(Action::WalletHold);
    nothing_kept(&app, "wallet.json");
    assert_eq!(app.toast_text(), Some("wallet.json is no longer in Files."));
}

/// A 1-of-2 over test seed 0's fingerprint and path with a key that is
/// not there, and a key of another seed.
fn forged_descriptor() -> String {
    let p = "m/48'/1'/0'/2'";
    two_keys(&forged_key(), &testkit::key(1, p))
}

/// Test seed 0's fingerprint and path over a key that is not there.
fn forged_key() -> String {
    let ours = testkit::key(0, "m/48'/1'/0'/2'");
    let elsewhere = testkit::key(0, "m/48'/1'/7'/2'");
    format!(
        "{}]{}",
        ours.split_once(']').unwrap().0,
        elsewhere.split_once(']').unwrap().1
    )
}

fn two_keys(a: &str, b: &str) -> String {
    checksummed(&format!("wsh(sortedmulti(1,{a}/<0;1>/*,{b}/<0;1>/*))"))
}

/// A MuSig2 wallet over two accounts of test seed 0.
fn musig_of_one_seed() -> String {
    let [a, b] = ["m/86'/1'/0'", "m/86'/1'/1'"].map(|p| testkit::key(0, p));
    checksummed(&format!("tr(musig({a},{b})/<0;1>/*)"))
}

/// A silent payments record of test seed 0's points: under its own
/// fingerprint with another's scan key, or with `under` its own points under
/// that fingerprint.
#[expect(
    clippy::disallowed_types,
    reason = "the test makes a record by hand, from the test seed's points and a made-up key"
)]
fn silent_record(under: Option<Fingerprint>) -> String {
    use faraday_bip::bitcoin::secp256k1::{Secp256k1, SecretKey};
    use faraday_bip::silent::Receiver;
    use faraday_bip::silent_wallet::SilentWallet;

    let c = testkit::compute();
    let mut app = with_files(Vec::new());
    loads_seed(&mut app, 0);
    let secp = Secp256k1::new();
    let r = Receiver::derive(&c, app.session.keys.master_for_tests(0), 0);
    let other = SecretKey::from_slice(&[7; 32]).unwrap().public_key(&secp);
    SilentWallet {
        network: app.session.network(),
        fingerprint: under.unwrap_or(app.session.keys[0].fingerprint),
        account: 0,
        scan: under.map_or(other, |_| r.scan_public_key(&secp)),
        spend: r.spend_public_key(),
        labels: 0,
    }
    .to_text()
}

/// Add to vault pressed on `name` is refused: nothing asked, kept or
/// loaded, the vault not changed and the file as it was. Its words.
fn add_refused(app: &mut Faraday, name: &str) -> String {
    let changes = app.vaults().open[0].changes;
    let file = app.inbox()[at(app, name)].bytes.to_vec();
    let said = app.toast_text().map(str::to_string);
    add(app, name);
    assert_eq!(app.sheet, None, "{name}: {:?}", app.toast_text());
    nothing_kept(app, name);
    assert_eq!(app.vaults().open[0].changes, changes, "{name}");
    assert_eq!(app.inbox()[at(app, name)].bytes.to_vec(), file, "{name}");
    let words = app.toast_text().map(str::to_string);
    assert_ne!(words, said, "{name}: Add says nothing");
    words.expect("Add says why")
}

/// Load pressed on `name` is refused. Its words.
fn load_refused(app: &mut Faraday, name: &str) -> String {
    app.press(Action::LoadWallet(at(app, name)));
    assert_eq!(app.sheet, None, "Load of {name}: {:?}", app.toast_text());
    assert!(app.session.wallets.is_empty(), "{name}: loaded");
    app.toast_text().expect("Load says why").to_string()
}

#[test]
fn add_to_vault_refuses_in_loads_words_what_load_refuses() {
    let elsewhere = silent_record(Some(Fingerprint([0, 0, 0, 1])));
    for (what, name, file, seed) in [
        ("a forged key", "forged.txt", forged_descriptor(), true),
        ("one seed twice", "musig.txt", musig_of_one_seed(), false),
        ("a silent record", "silent.txt", silent_record(None), true),
        ("another key's silent record", "silent.txt", elsewhere, true),
    ] {
        let mut app = empty_vault_with(name, file.as_bytes());
        if seed {
            loads_seed(&mut app, 0);
        }
        let by_add = add_refused(&mut app, name);
        assert_eq!(load_refused(&mut app, name), by_add, "{what}");
    }
}

/// The export, its fingerprint made a loaded key's, so that its silent
/// payments account is not that key's: refused once the account dialog has
/// run Add to vault again, in the words Load comes to the same way.
#[test]
fn add_to_vault_refuses_after_the_account_dialog_what_load_refuses() {
    use faraday_core::wallet::Account;
    let mut app = empty_vault_with("later.txt", b"read below");
    loads_seed(&mut app, 0);
    app.press(Action::Back);
    let fp = app.session.keys[0].fingerprint.to_string().to_uppercase();
    let export = CC_EXPORT.replace("0F056943", &fp);
    assert_ne!(export, CC_EXPORT);
    read_in(&mut app, "cc.json", export.as_bytes());
    let changes = app.vaults().open[0].changes;
    add(&mut app, "cc.json");
    assert_eq!(app.sheet, Some(Sheet::Account), "{:?}", app.toast_text());
    app.press(Action::LoadAccount(Some(Account::Silent)));
    assert_eq!(app.sheet, None, "{:?}", app.toast_text());
    nothing_kept(&app, "cc.json");
    assert_eq!(app.vaults().open[0].changes, changes);
    let by_add = app.toast_text().expect("Add says why").to_string();
    app.press(Action::LoadWallet(at(&app, "cc.json")));
    assert_eq!(app.sheet, Some(Sheet::Account), "{:?}", app.toast_text());
    app.press(Action::LoadAccount(Some(Account::Silent)));
    assert!(app.session.wallets.is_empty(), "loaded");
    assert_eq!(app.toast_text(), Some(by_add.as_str()));
}

#[test]
fn add_to_vault_keeps_with_nothing_loaded_what_only_a_loaded_key_shows() {
    let forged = forged_descriptor();
    let mut app = empty_vault_with("forged.txt", forged.as_bytes());
    asked_before_keeping(&mut app, "forged.txt");
    app.press(Action::WalletHold);
    assert_eq!(wallet_items(&app), [forged], "{:?}", app.toast_text());

    let record = silent_record(None);
    let mut app = empty_vault_with("silent.txt", record.as_bytes());
    add(&mut app, "silent.txt");
    assert_eq!(app.sheet, None, "{:?}", app.toast_text());
    assert_eq!(wallet_items(&app), [record], "{:?}", app.toast_text());
}

#[test]
fn a_forged_key_loaded_under_add_to_vaults_sheet_is_refused_at_the_hold() {
    let d = forged_descriptor();
    let mut app = empty_vault_with("forged.txt", d.as_bytes());
    asked_before_keeping(&mut app, "forged.txt");
    let changes = app.vaults().open[0].changes;
    loads_seed(&mut app, 0);
    app.press(Action::WalletHold);
    nothing_kept(&app, "forged.txt");
    assert_eq!(app.vaults().open[0].changes, changes);
    let at_the_hold = app.toast_text().expect("the hold says why").to_string();
    assert_eq!(load_refused(&mut app, "forged.txt"), at_the_hold);
}

/// Test seed 0 loaded and waiting for its backup, and its key under the
/// origin fingerprint `00000001`, which no key here has.
fn waiting_under_another_fingerprint(app: &mut Faraday) -> String {
    loads_seed(app, 0);
    let fp = app.session.keys[0].fingerprint;
    testkit::mark_made_here(app, fp);
    app.press(Action::Back);
    let key = testkit::key(0, "m/48'/1'/0'/2'");
    format!("[00000001{}", &key[9..])
}

/// A key waiting for its backup under another fingerprint: Add to vault
/// asks and keeps it. Its hold does not stand in for Load's sheet, which
/// Load comes to once the backup is done.
#[test]
fn add_to_vault_keeps_a_waiting_keys_wallet_and_load_still_asks_once_backed_up() {
    let mut app = empty_vault_with("later.txt", b"read below");
    let waiting = waiting_under_another_fingerprint(&mut app);
    let d = two_keys(&waiting, &testkit::key(1, "m/48'/1'/0'/2'"));
    read_in(&mut app, "waiting.txt", d.as_bytes());
    asked_before_keeping(&mut app, "waiting.txt");
    app.press(Action::WalletHold);
    assert_eq!(wallet_items(&app), [d], "{:?}", app.toast_text());
    let words = load_refused(&mut app, "waiting.txt");
    assert!(words.contains("not backed up"), "{words}");
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    app.press(Action::Back);
    load_asks(&mut app, "waiting.txt");
}

/// A waiting key written before a forged one does not hide it.
#[test]
fn add_to_vault_refuses_a_forged_key_on_either_side_of_a_waiting_one() {
    let mut app = empty_vault_with("later.txt", b"read below");
    let waiting = waiting_under_another_fingerprint(&mut app);
    let forged = forged_key();
    let after = two_keys(&forged, &waiting);
    let before = two_keys(&waiting, &forged);
    assert_ne!(after, before, "the order written is kept");
    read_in(&mut app, "after.txt", after.as_bytes());
    read_in(&mut app, "before.txt", before.as_bytes());
    let words = add_refused(&mut app, "after.txt");
    assert_eq!(load_refused(&mut app, "after.txt"), words);
    // Load stops at the first key it refuses.
    let first = load_refused(&mut app, "before.txt");
    assert!(first.contains("not backed up"), "{first}");
    assert_eq!(add_refused(&mut app, "before.txt"), words);
}

#[test]
fn a_vault_full_at_add_to_vaults_hold_keeps_nothing_and_load_still_asks() {
    let d = planted_descriptor();
    let mut app = empty_vault_with("planted.txt", d.as_bytes());
    asked_before_keeping(&mut app, "planted.txt");
    let id = app.vaults().open[0].id;
    // The most a vault may hold, lowered to just over what it holds. Past
    // a limit of nothing the size is counted, not bounded.
    testkit::vault_edit(&mut app, id, |s| s.lower_limit(0));
    let used = app.vaults().records(id).used(&testkit::compute());
    testkit::vault_edit(&mut app, id, |s| s.lower_limit(used + 20));
    app.press(Action::WalletHold);
    nothing_kept(&app, "planted.txt");
    assert_eq!(app.toast_text(), Some(EN.vaults_slot_too_small));
    load_asks(&mut app, "planted.txt");
}

#[test]
fn backups_know_the_vault_has_the_wallet_as_soon_as_add_to_vaults_hold() {
    let d = planted_descriptor();
    let mut app = empty_vault_with("planted.txt", d.as_bytes());
    asked_before_keeping(&mut app, "planted.txt");
    app.press(Action::WalletHold);
    app.press(Action::LoadWallet(at(&app, "planted.txt")));
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    assert_eq!(
        app.backup_line(&testkit::compute(), 0).as_str(),
        EN.backups_no_plan_kept
    );
}

/// A `tr()` wallet whose one tree key is loaded and whose key path a bare
/// public key holds.
#[test]
fn add_to_vault_asks_about_a_key_path_another_key_holds() {
    // secp256k1's generator, x only: a public key that is not BIP 341's H.
    let bare = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    let key = testkit::key(0, "m/86'/1'/0'");
    let d = checksummed(&format!("tr({bare},pk({key}/<0;1>/*))"));
    let mut app = empty_vault_with("ours.txt", d.as_bytes());
    loads_seed(&mut app, 0);
    asked_before_keeping(&mut app, "ours.txt");
}

#[test]
fn escape_and_lock_in_add_to_vaults_sheet_keep_nothing() {
    let d = planted_descriptor();
    let mut app = empty_vault_with("planted.txt", d.as_bytes());
    asked_before_keeping(&mut app, "planted.txt");
    app.event(Event::Key(Key::Escape));
    assert_eq!(app.sheet, None);
    assert!(app.session.wallet_asked().is_none(), "a sheet waits");
    nothing_kept(&app, "planted.txt");
    asked_before_keeping(&mut app, "planted.txt");
    app.press(Action::Lock);
    assert!(app.restart_requested(), "{:?}", app.toast_text());
    assert!(app.session.wallet_asked().is_none(), "a sheet waits");
    app.press(Action::WalletHold);
    assert!(app.session.wallets.is_empty(), "loaded");
}

/// A wallet in each of the other forms a policy takes: Add to vault asks,
/// and what it keeps loads after power off as the wallet Load gives.
#[test]
fn add_to_vault_keeps_each_form_of_policy_as_load_reads_it() {
    let p = "m/48'/1'/0'/2'";
    let [a, b, c] = [1, 2, 0].map(|n| testkit::key(n, p));
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "taproot-multisig")
        .expect("kit");
    let musig = testkit::musig_policy()
        .expect("the MuSig2 wallet")
        .to_descriptor_checksummed();
    let forms: [(&str, String); 4] = [
        (
            "miniscript.txt",
            format!("wsh(or_d(pk({a}/<0;1>/*),and_v(v:pkh({b}/<0;1>/*),older(1000))))"),
        ),
        (
            "tree.txt",
            format!("tr({a}/<0;1>/*,{{pk({b}/<0;1>/*),pk({c}/<0;1>/*)}})"),
        ),
        ("multi_a.txt", kit.descriptor.to_string()),
        ("musig.txt", musig.to_string()),
    ];
    let loaded = |app: &Faraday| {
        assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
        let policy = app.session.wallets[0].policy();
        policy.to_descriptor_checksummed().to_string()
    };
    for (name, file) in forms {
        let mut app = empty_vault_with(name, file.as_bytes());
        load_asks(&mut app, name);
        app.press(Action::WalletHold);
        let by_load = loaded(&app);

        let mut app = empty_vault_with(name, file.as_bytes());
        asked_before_keeping(&mut app, name);
        app.press(Action::WalletHold);
        let kept = wallet_items(&app);
        assert_eq!(kept.len(), 1, "{name}: {:?}", app.toast_text());
        let mut app = sealed_and_reopened(app);
        assert_eq!(wallet_items(&app), kept, "{name}: after power off");
        app.press(Action::Vault(V::LoadAll(0)));
        assert_eq!(app.sheet, None, "{name}: {:?}", app.toast_text());
        assert_eq!(loaded(&app), by_load, "{name}");
    }
}

/// A wallet of one key is not asked about: the vault's record has the name
/// a load gives it. A label refused as a name, or reading as the name of no
/// name, gives that name; with no label it is the file's name in words.
#[test]
fn the_vaults_wallet_of_one_key_has_the_name_a_load_gives() {
    let key = testkit::key(1, "m/84'/1'/0'");
    let d = checksummed(&format!("wpkh({key}/<0;1>/*)"));
    let json = |label: &str| format!(r#"{{"label":"{label}","blockheight":0,"descriptor":"{d}"}}"#);
    // A word of the seed list over and over reads as seed words.
    let words = "shop ".repeat(12);
    for (name, file, want) in [
        (
            "export.json",
            json(words.trim_end()),
            EN.wallet_default_name,
        ),
        (
            "export.json",
            json(EN.wallet_default_name),
            EN.wallet_default_name,
        ),
        ("export.txt", d.clone(), "Export"),
    ] {
        let mut app = empty_vault_with(name, file.as_bytes());
        add(&mut app, name);
        assert_eq!(item_names(&app), [want], "{file}: {:?}", app.toast_text());
        app.press(Action::LoadWallet(at(&app, name)));
        assert_eq!(&*app.session.wallets[0].name, want, "{file}");
    }
}

/// The names of the open vault's wallet items.
fn item_names(app: &Faraday) -> Vec<String> {
    app.vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .of(kind::WALLET)
        .map(|(_, r)| r.text(field::WALLET_NAME).unwrap_or_default().to_string())
        .collect()
}

/// The name on the confirmation sheet that is open.
fn asked_name(app: &Faraday) -> String {
    let asked = app.session.wallet_asked().expect("a sheet waits");
    asked.name.to_string()
}

/// A file with no name in it: Add to vault's sheet and the vault's record
/// say what Load's sheet says, the file's name in words, cut as a name is.
#[test]
fn add_to_vault_names_a_wallet_as_load_does() {
    let d = testkit::savings();
    let long = format!("{}.txt", "zzqx-".repeat(24));
    for (name, want) in [
        ("savings.txt", Some("Savings")),
        ("my-old_cold-wallet.txt", Some("My Old Cold")),
        (long.as_str(), None),
    ] {
        let mut app = empty_vault_with(name, d.as_bytes());
        load_asks(&mut app, name);
        let by_load = asked_name(&app);
        match want {
            Some(want) => assert_eq!(by_load, want, "{name}"),
            None => assert!(by_load.starts_with("Zzqx Zzqx"), "{by_load}"),
        }
        app.press(Action::WalletCancel);
        asked_before_keeping(&mut app, name);
        assert_eq!(asked_name(&app), by_load, "{name}");
        app.press(Action::WalletHold);
        assert_eq!(
            item_names(&app),
            [by_load],
            "{name}: {:?}",
            app.toast_text()
        );
    }
}
