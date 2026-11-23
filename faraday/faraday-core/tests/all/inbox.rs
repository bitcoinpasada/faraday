//! What Files offers for each kind of file copied in: a text file, a
//! seed's words or a descriptor kept in the open vault with one press,
//! and the shares of a split backup put back together into the wallet
//! from Files itself.

use crate::common::RecordsOf as _;
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
        app.sheet = Some(Sheet::Import);
        texts_at(&mut app, "boot import sheet", &mut all);
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
