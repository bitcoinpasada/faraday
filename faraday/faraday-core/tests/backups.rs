//! The Backups screen and the wallet card's backup line
//! (`docs/SIMPLIFY.md` §5): every wallet known, its backup map a line per
//! place, each place with what this device saw of it. A paper place
//! reads checked only once a copy of it was checked here; the vault line
//! is sealed and names the stick once a visit wrote it; a wallet with no
//! plan offers Back up; a wallet seen only in a locked vault is listed
//! from what was remembered of it.

use faraday_core::backup::Tone;
use faraday_core::backups::Line;
use faraday_core::plan::Tag;
use faraday_core::seeds::SeedsAction as S;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent, bstep, testkit};
use osk_shell_api::{
    App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware, TouchPhase,
};

type Files = Vec<(String, Vec<u8>)>;

fn shown(inbox: Files, outbox: Files, kept: Files) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1920,
        height: 1080,
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
        inbox,
        outbox,
        kept,
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    app.vaults.ms_per_unit = Some(180);
    let _ = app.frame();
    app
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
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

/// Presses `action` where the last frame drew it.
fn tap(app: &mut Faraday, action: Action) {
    let _ = app.frame();
    let (x, y) = app
        .where_offered(action)
        .unwrap_or_else(|| panic!("{action:?} is not on screen"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
}

/// Test key 1 typed and a one-key wallet made from it.
fn with_a_wallet() -> Faraday {
    let mut app = shown(Vec::new(), Vec::new(), Vec::new());
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    let fp = app.session.keys[0].master.fingerprint().0;
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    assert_eq!(app.session.wallets.len(), 1, "no wallet was made");
    app
}

/// A vault made here and open.
fn open_a_vault(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(app, "test phrase");
    }
    app.press(Action::Vault(V::CGo));
    settle(app, |a| a.screen == Screen::Unlock);
    type_text(app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults.open.is_empty());
    assert_eq!(app.vaults.open.len(), 1, "the vault did not open");
}

/// The copy of the seed on screen typed back by its numbers.
fn type_the_copy(app: &mut Faraday) {
    let mn = osk_bip::bip39::Mnemonic::parse(
        osk_bip::bip39::Language::English,
        &testkit::test_words(testkit::TEST_SEEDS[0].0),
    )
    .unwrap();
    let digits = osk_codec::seedqr::to_digits(&mn);
    let digits = digits.expose().as_bytes().to_vec();
    for c in digits {
        app.event(Event::Key(Key::Char(char::from(c))));
    }
}

/// The wallet backed up with Paper and vault, its copy checked `copies`
/// times, the seed and the wallet saved into the open vault.
fn backed_up(copies: usize) -> Faraday {
    let mut app = with_a_wallet();
    open_a_vault(&mut app);
    app.press(Action::Backup(0));
    app.press(Action::BPreset(1));
    app.press(Action::BChecklist);
    if app.backup.as_ref().and_then(|b| b.open) != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
    app.press(Action::BReveal);
    app.press(Action::BCheck);
    for k in 0..copies {
        if k > 0 {
            app.press(Action::BCheckClear);
        }
        type_the_copy(&mut app);
    }
    app.press(Action::BCheck);
    app.press(Action::BVaultSave(0));
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::AddKind(1)));
    app.press(Action::Vault(V::SaveWallet(0)));
    app
}

/// The Backups screen's lines for the one wallet listed.
fn lines(app: &mut Faraday) -> Vec<Line> {
    app.press(Action::Nav(Screen::Backups));
    let _ = app.frame();
    let all = app.backups();
    assert_eq!(all.len(), 1, "one wallet listed: {all:?}");
    all[0].lines.clone().expect("the wallet has a plan")
}

fn line<'a>(lines: &'a [Line], place: &str) -> &'a Line {
    lines
        .iter()
        .find(|l| l.place == place)
        .unwrap_or_else(|| panic!("no line for {place}: {lines:?}"))
}

#[test]
fn a_wallet_backed_up_with_paper_and_vault_shows_two_paper_places_checked_and_the_vault_sealed() {
    let mut app = backed_up(2);
    let lines = lines(&mut app);
    for place in ["Place 1", "Place 2"] {
        let l = line(&lines, place);
        assert_eq!((l.state.as_str(), l.tone), ("Checked", Tone::Ok), "{place}");
        assert_eq!(l.tag, Some(Tag::Secret), "{place} holds the seed");
    }
    let vault = line(&lines, "Vault");
    assert_eq!(vault.tag, Some(Tag::Sealed));
    let texts = app.drawn_texts();
    assert_eq!(
        texts.iter().filter(|t| *t == "Checked").count(),
        2,
        "two paper places checked: {texts:?}"
    );
    assert!(texts.iter().any(|t| t == "sealed"), "{texts:?}");
}

/// One check is one copy: the second place keeps a copy no check on this
/// device has seen.
#[test]
fn one_check_marks_one_paper_place() {
    let mut app = backed_up(1);
    let lines = lines(&mut app);
    assert_eq!(line(&lines, "Place 1").state, "Checked");
    assert_eq!(line(&lines, "Place 2").state, "Not checked");
}

/// What the lock saved: the Inbox, the Outbox and the kept state.
fn saved(app: &mut Faraday) -> (Files, Files, Files) {
    let mut last = (Vec::new(), Vec::new(), Vec::new());
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox,
            outbox,
            kept,
        } = c
        {
            last = (inbox, outbox, kept);
        }
    }
    last
}

/// Answers every write the visit sends as written and matched.
fn answer_writes(app: &mut Faraday) {
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Write { stick, name, .. } = c {
            app.storage(StorageEvent::Written {
                stick,
                wrote_as: name.clone(),
                name,
            });
        }
    }
}

#[test]
fn after_the_vault_is_written_the_vault_line_names_the_stick() {
    let mut app = backed_up(2);
    let file = app.vault_files()[0].name.clone();
    let before = line(&lines(&mut app), "Vault").state.clone();
    assert_eq!(before, format!("{file} · for the stick"));

    // Sealed at the lock; the next process writes it.
    app.press(Action::Lock);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut app = shown(inbox, outbox, kept);
    app.storage(StorageEvent::Sticks(vec![faraday_core::StickInfo {
        id: "S".into(),
        label: "VAULTSTICK".into(),
        boot: false,
        files: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Visit));
    app.press(Action::VisitWrite);
    answer_writes(&mut app);
    app.storage(StorageEvent::Sticks(Vec::new()));

    let lines = lines(&mut app);
    let vault = line(&lines, "Vault");
    assert!(
        vault.state.starts_with(&format!("{file} · on VAULTSTICK")),
        "{vault:?}"
    );
    assert_eq!(vault.tone, Tone::Ok);
    // What was checked before the lock still reads checked.
    assert_eq!(line(&lines, "Place 2").state, "Checked");
}

#[test]
fn a_wallet_with_no_plan_offers_back_up() {
    let mut app = with_a_wallet();
    app.press(Action::Nav(Screen::Backups));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "No backup plan"), "{texts:?}");
    tap(&mut app, Action::Backup(0));
    assert_eq!(app.screen, Screen::Backup);
}

#[test]
fn a_wallet_seen_only_in_a_locked_vaults_summary_is_listed_from_it() {
    let mut app = with_a_wallet();
    let name = app.session.wallets[0].name.clone();
    open_a_vault(&mut app);
    app.press(Action::Vault(V::AddKind(1)));
    app.press(Action::Vault(V::SaveWallet(0)));
    app.press(Action::Lock);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut app = shown(inbox, outbox, kept);
    assert!(app.session.wallets.is_empty());
    app.press(Action::Nav(Screen::Backups));
    let texts = app.drawn_texts();
    assert!(texts.contains(&name), "{name}: {texts:?}");
    assert!(texts.iter().any(|t| t == "No backup plan"), "{texts:?}");
}

#[test]
fn the_wallet_card_of_a_backed_up_wallet_says_where_its_backup_is() {
    let mut app = backed_up(2);
    let file = app.vault_files()[0].name.clone();
    app.press(Action::Nav(Screen::Wallets));
    let texts = app.drawn_texts();
    let want = format!("Backup: paper ×2 checked · {file} · for the stick");
    assert!(texts.contains(&want), "{want}: {texts:?}");
    tap(&mut app, Action::BackupsOf(0));
    assert_eq!(app.screen, Screen::Backups);
}

#[test]
fn the_wallet_card_of_a_fresh_wallet_says_it_is_not_backed_up() {
    let mut app = with_a_wallet();
    app.press(Action::Nav(Screen::Wallets));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Not backed up"), "{texts:?}");
    assert!(app.offers(Action::Backup(0)), "the card offers Back up");
}
