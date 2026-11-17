//! The Backups screen and the wallet card's backup line:
//! every wallet known, its backup map a line per
//! place, each place with what this device saw of it. A paper place
//! reads checked only once a copy of it was checked here; the vault line
//! is sealed and names the stick once a visit wrote it; a wallet with no
//! plan offers Back up; a wallet seen only in a locked vault is listed
//! from what was remembered of it.

use crate::common::type_text;
use faraday_core::backups::Line;
use faraday_core::plan::Tag;
use faraday_core::seeds::SeedsAction as S;
use faraday_core::testkit::Plain;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet_backup::Tone;
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent, testkit};
use faraday_shell_api::{
    App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware, TouchPhase,
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
        inbox: faraday_core::files_from(inbox),
        outbox: faraday_core::files_from(outbox),
        kept: faraday_core::kept_from(kept),
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    let _ = app.frame();
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
    faraday_core::testkit::keep_published(&mut app);
    let fp = app.session.keys[0].fingerprint.0;
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
    settle(app, |a| !a.vaults().open.is_empty());
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
}

/// The copy of the seed on screen typed back as its words.
fn type_the_copy(app: &mut Faraday) {
    testkit::type_words(app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
}

/// The wallet backed up with Paper and vault, its copy checked `copies`
/// times, the seed and the wallet saved into the open vault.
fn backed_up(copies: usize) -> Faraday {
    let mut app = with_a_wallet();
    open_a_vault(&mut app);
    app.press(Action::Backup(0));
    preset_saved(&mut app, 2);
    app.press(Action::BChecklist);
    if app.backup().and_then(|b| b.open) != Some(faraday_core::plan::Item::Copy(0)) {
        app.press(Action::BStep(faraday_core::plan::Item::Copy(0)));
    }
    app.press(Action::BReveal);
    // Each copy past the first is checked again from the start.
    for _ in 0..copies {
        app.press(Action::BCheck);
        type_the_copy(&mut app);
    }
    app.press(Action::BVaultSave(0));
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::AddKind(1)));
    app.press(Action::Vault(V::SaveWallet(0)));
    app
}

/// The Backups screen's lines for the one wallet listed.
fn lines(app: &mut Faraday) -> Vec<Line> {
    let c = testkit::compute();
    app.press(Action::Nav(Screen::Backups));
    let _ = app.frame();
    let all = app.backups_now(&c);
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
            last = (
                faraday_core::testkit::plain_files(inbox),
                faraday_core::testkit::plain_files(outbox),
                faraday_core::testkit::plain_kept(kept),
            );
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
    let file = String::from(app.vault_files()[0].name.as_str());
    let before = line(&lines(&mut app), "Vault").state.clone();
    assert_eq!(&*before, format!("{file} · for the stick"));

    // Sealed at the lock; the next process writes it.
    crate::common::lock(&mut app);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut app = shown(inbox, outbox, kept);
    app.storage(StorageEvent::Sticks(vec![faraday_core::StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("VAULTSTICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Files));
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

/// A locked vault's summary says a seed is kept there by the seed's
/// whole master id: the vault line names the vault for the seed it held,
/// says nothing is verified from a summary with no ids, and does not
/// name it for another seed that only shares the fingerprint. A real
/// four-byte collision cannot be made here, so the summary's entry of
/// another seed is given this seed's fingerprint, as a collision would
/// leave it.
#[test]
fn a_locked_vault_is_named_for_a_seed_it_held_and_not_for_another_with_its_fingerprint() {
    use faraday_core::wallet::fp_text;
    let mut app = backed_up(2);
    // Test key 2 kept in the vault too.
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    assert_eq!(app.session.keys.len(), 2, "test key 2 loaded");
    let (fp1, fp2) = (
        String::from(&*fp_text(app.session.keys[0].fingerprint)),
        String::from(&*fp_text(app.session.keys[1].fingerprint)),
    );
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::SaveKey(1)));
    let file = String::from(app.vault_files()[0].name.as_str());
    crate::common::lock(&mut app);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut app = shown(inbox, outbox, kept);
    assert!(app.vaults().open.is_empty(), "the vault is locked");
    assert!(app.session.keys.is_empty(), "no seed is loaded");
    let vault = line(&lines(&mut app), "Vault").clone();
    assert!(vault.state.starts_with(&format!("{file} · ")), "{vault:?}");

    // A summary kept before the ids were: nothing is verified.
    let ids = app.vaults().summaries[0].ids.clone();
    faraday_core::testkit::change_vault_summaries(&mut app, |summaries| {
        for id in &mut summaries[0].ids {
            *id = None;
        }
    });
    let vault = line(&lines(&mut app), "Vault").clone();
    assert_eq!(
        (vault.state.as_str(), vault.tone),
        ("A vault is locked", Tone::Dim)
    );
    faraday_core::testkit::change_vault_summaries(&mut app, |summaries| summaries[0].ids = ids);

    // Test key 2's entry under test key 1's fingerprint, and test key 1
    // gone from the summary.
    faraday_core::testkit::change_vault_summaries(&mut app, |summaries| {
        let s = &mut summaries[0];
        let i1 = s
            .keys
            .iter()
            .position(|k| k.as_str() == fp1)
            .expect("test key 1");
        let i2 = s
            .keys
            .iter()
            .position(|k| k.as_str() == fp2)
            .expect("test key 2");
        s.keys[i2] = faraday_text::Text::of(&fp1);
        s.keys.remove(i1);
        s.ids.remove(i1);
    });
    let vault = line(&lines(&mut app), "Vault").clone();
    assert!(
        !vault.state.contains(&file),
        "a fingerprint alone does not make another seed kept: {vault:?}"
    );
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
fn a_wallet_card_opens_the_wallet() {
    let mut app = with_a_wallet();
    app.press(Action::Nav(Screen::Backups));
    tap(&mut app, Action::OpenWallet(0));
    assert_eq!(app.screen, Screen::Wallets);
}

#[test]
fn a_place_row_opens_its_places_sheet() {
    let mut app = backed_up(2);
    app.press(Action::Nav(Screen::Backups));
    let at = {
        let c = testkit::compute();
        let all = app.backups_now(&c);
        all[0]
            .lines
            .as_ref()
            .and_then(|l| l.iter().find(|l| l.paper))
            .and_then(|l| l.at)
            .expect("a paper place")
    };
    tap(&mut app, Action::BackupsPlace(0, at));
    assert_eq!(app.screen, Screen::Backups);
    assert!(matches!(
        app.chart(),
        Some((_, faraday_core::glance::sheet::Target::Node(_)))
    ));
}

#[test]
fn with_no_wallet_backups_offers_make_and_restore() {
    let mut app = shown(Vec::new(), Vec::new(), Vec::new());
    app.press(Action::Nav(Screen::Backups));
    let _ = app.frame();
    assert!(app.offers(Action::CreateWallet));
    assert!(app.offers(Action::RestoreWallet));
    tap(&mut app, Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
}

#[test]
fn a_wallet_in_a_locked_vault_offers_unlock() {
    let mut app = with_a_wallet();
    open_a_vault(&mut app);
    app.press(Action::Vault(V::AddKind(1)));
    app.press(Action::Vault(V::SaveWallet(0)));
    crate::common::lock(&mut app);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut app = shown(inbox, outbox, kept);
    app.press(Action::Nav(Screen::Backups));
    let c = testkit::compute();
    let (_, unlock) = app.backups_now(&c)[0].unlock.clone().expect("an unlock");
    tap(&mut app, unlock);
    assert_eq!(app.screen, Screen::Unlock);
}

#[test]
fn a_wallet_seen_only_in_a_locked_vaults_summary_is_listed_from_it() {
    let mut app = with_a_wallet();
    let name = app.session.wallets[0].name.as_str().to_string();
    open_a_vault(&mut app);
    app.press(Action::Vault(V::AddKind(1)));
    app.press(Action::Vault(V::SaveWallet(0)));
    crate::common::lock(&mut app);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut app = shown(inbox, outbox, kept);
    assert!(app.session.wallets.is_empty());
    app.press(Action::Nav(Screen::Backups));
    let texts = app.drawn_texts().plain();
    assert!(texts.contains(&name), "{name}: {texts:?}");
    assert!(texts.iter().any(|t| t == "No backup plan"), "{texts:?}");
}

#[test]
fn the_wallet_card_of_a_backed_up_wallet_says_where_its_backup_is() {
    let mut app = backed_up(2);
    let file = String::from(app.vault_files()[0].name.as_str());
    app.press(Action::Nav(Screen::Wallets));
    let texts = app.drawn_texts().plain();
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

/// Home's count of wallets with no backup plan, and the count the
/// Backups screen's list gives, as the same wallets.
fn counts_agree(app: &Faraday) -> (usize, usize) {
    let c = testkit::compute();
    let all = app.backups_now(&c);
    let counts = app.backup_counts_now(&c);
    let without = all.iter().filter(|e| e.lines.is_none()).count();
    assert_eq!(counts, (all.len(), without), "{all:?}");
    counts
}

#[test]
fn home_counts_a_wallet_without_a_plan_until_its_backup_is_planned() {
    let mut app = with_a_wallet();
    app.press(Action::Nav(Screen::Home));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "1 not backed up"), "{texts:?}");
    assert_eq!(counts_agree(&app), (1, 1));

    // A vault opening does not give the wallet a plan.
    open_a_vault(&mut app);
    app.press(Action::Nav(Screen::Home));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "1 not backed up"), "{texts:?}");

    // A plan made in the backup flow does.
    app.press(Action::Backup(0));
    preset_saved(&mut app, 2);
    app.press(Action::BChecklist);
    app.press(Action::Nav(Screen::Home));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "1 with a plan"), "{texts:?}");
    assert!(
        !texts.iter().any(|t| t.ends_with("not backed up")),
        "{texts:?}"
    );
    assert_eq!(counts_agree(&app), (1, 0));
}

#[test]
fn home_counts_a_wallet_known_only_from_a_vault_or_a_locked_vaults_summary() {
    let mut app = with_a_wallet();
    open_a_vault(&mut app);
    app.press(Action::Vault(V::AddKind(1)));
    app.press(Action::Vault(V::SaveWallet(0)));
    // The wallet is loaded and in the vault: one wallet, listed once.
    assert_eq!(counts_agree(&app), (1, 1));
    crate::common::lock(&mut app);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut app = shown(inbox, outbox, kept);
    assert!(app.session.wallets.is_empty());
    assert_eq!(counts_agree(&app), (1, 1));
    app.press(Action::Nav(Screen::Home));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "1 not backed up"), "{texts:?}");
}

#[test]
fn the_wallet_card_follows_the_backup_as_it_is_planned() {
    let mut app = with_a_wallet();
    app.press(Action::Nav(Screen::Wallets));
    assert!(app.drawn_texts().iter().any(|t| t == "Not backed up"));
    open_a_vault(&mut app);
    app.press(Action::Backup(0));
    preset_saved(&mut app, 2);
    app.press(Action::BChecklist);
    app.press(Action::Nav(Screen::Wallets));
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t == "Not backed up"), "{texts:?}");
    assert!(texts.iter().any(|t| t.starts_with("Backup: ")), "{texts:?}");
}

/// Back up's plan as preset `k` (by `Preset::ALL`) fills it over a place
/// per key, two for a single key: as a plan saved before plans kept at
/// most one place per key has it (`docs/BACKUP-FLOW.md` §17.19).
fn preset_saved(app: &mut Faraday, k: u8) {
    let w = app.backup().unwrap().wallet;
    let shape = app.plan_shape_now(w);
    let base = if shape.multi() { shape.seeds.len() } else { 2 };
    let preset = faraday_core::plan::Preset::ALL[usize::from(k)];
    faraday_core::testkit::set_backup_answers(
        app,
        faraday_core::plan::Answers::preset_at(&shape, preset, base),
    );
}

/// A page scrolled: the frames drawn are moved ones.
fn scrolled(app: &mut Faraday) {
    for _ in 0..3 {
        app.event(Event::Wheel {
            x: 960,
            y: 540,
            dy: 40,
        });
        let _ = app.frame();
    }
}

/// The Backups screen and the wallet's chart keep their lists while a page
/// scrolls; a plan made and a backup checked each show on the next frame.
#[test]
fn the_backups_screen_and_the_wallet_chart_follow_the_backup_while_a_page_scrolls() {
    let mut app = with_a_wallet();
    app.press(Action::Nav(Screen::Backups));
    scrolled(&mut app);
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "No backup plan"), "{texts:?}");
    app.press(Action::Nav(Screen::Wallets));
    scrolled(&mut app);
    open_a_vault(&mut app);
    app.press(Action::Backup(0));
    preset_saved(&mut app, 2);
    app.press(Action::BChecklist);
    app.press(Action::Nav(Screen::Backups));
    scrolled(&mut app);
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t == "No backup plan"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Place 1"), "{texts:?}");
    app.press(Action::Nav(Screen::Wallets));
    scrolled(&mut app);
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t.starts_with("Backup: ")), "{texts:?}");
}

/// A copy checked while the Backups screen was scrolled shows as checked
/// on the next frame.
#[test]
fn a_copy_checked_shows_on_the_backups_screen_after_it_was_scrolled() {
    let mut app = backed_up(1);
    app.press(Action::Nav(Screen::Backups));
    scrolled(&mut app);
    let texts = app.drawn_texts();
    assert_eq!(
        texts.iter().filter(|t| *t == "Checked").count(),
        1,
        "{texts:?}"
    );
    app.press(Action::Backup(0));
    app.press(Action::BStep(faraday_core::plan::Item::Copy(0)));
    app.press(Action::BReveal);
    app.press(Action::BCheck);
    type_the_copy(&mut app);
    app.press(Action::Nav(Screen::Backups));
    scrolled(&mut app);
    let texts = app.drawn_texts();
    assert_eq!(
        texts.iter().filter(|t| *t == "Checked").count(),
        2,
        "{texts:?}"
    );
}

/// A wallet of a key loaded from SLIP-39 shares or a master seed (no
/// words), planned Paper and vault, with the vault open.
fn shares_wallet() -> Faraday {
    use std::str::FromStr;
    let c = testkit::compute();
    let mut app = shown(Vec::new(), Vec::new(), Vec::new());
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    let fp = app.session.keys.add_seed(&c, &[7u8; 32], "").unwrap();
    let path = faraday_bip::bitcoin::bip32::DerivationPath::from_str("m/84'/1'/0'").unwrap();
    let pos = app.session.keys.position(fp).unwrap();
    let x = app.session.keys.xpub_at(&c, pos, &path).unwrap();
    let hex = String::from_utf8_lossy(&fp.to_hex()).into_owned();
    let d = format!("wpkh([{hex}/84'/1'/0']{x}/<0;1>/*)");
    app.session
        .add_wallet(
            &c,
            "Test",
            &d,
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .expect("reads");
    open_a_vault(&mut app);
    app.press(Action::Backup(0));
    preset_saved(&mut app, 2);
    app
}

/// A key kept as shares is not saved in the vault until its record is: the
/// Backups screen says "Not saved yet" first, then the vault's state.
#[test]
fn the_vault_line_of_a_key_kept_as_shares_follows_the_key_in_the_vault() {
    let mut app = shares_wallet();
    app.press(Action::BChecklist);
    let before = line(&lines(&mut app), "Vault").clone();
    assert_eq!(before.state, "Not saved yet", "{before:?}");
    app.press(Action::BVaultSave(0));
    let kept: usize = app
        .vaults()
        .open
        .iter()
        .map(|o| {
            use crate::common::RecordsOf as _;
            app.vaults()
                .store()
                .contents_for_tests(o.id)
                .of(faraday_vault::records::kind::KEY)
                .count()
        })
        .sum();
    assert_eq!(kept, 1, "the vault did not take the key");
    let held = line(&lines(&mut app), "Vault").clone();
    assert_eq!(held.tone, Tone::Warn, "{held:?}");
    assert!(held.state.ends_with("for the stick"), "{held:?}");
}
