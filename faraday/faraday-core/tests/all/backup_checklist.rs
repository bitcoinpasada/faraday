//! The backup starts with a plan and goes on to a checklist of only what
//! the plan needs. Each item is done by what it does: the template or the
//! sheet in the Outbox, the copy by hand matched, the seed and the wallet
//! in the vault; the envelopes alone by a press. A place's name is kept
//! in the vault, where the next backup of the wallet finds the plan, and
//! in no file. The blank template has a line per place and no name.

use crate::common::{files_stick, form, no_words, put, type_text};
use faraday_core::board::{Thing, Zone};
use faraday_core::plan::Item;
use faraday_core::seeds::SeedsAction as S;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, BStage, Faraday, Screen, StorageEvent, bstep, plan, qrow, qstep};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};
use faraday_vault::records::{field, kind};

const NAME: &str = "Harbour cellar";

fn device(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
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
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    let _ = app.frame();
    app
}

fn vault_file() -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .unwrap()
}

fn unlock(app: &mut Faraday) {
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the test vault did not unlock");
}

/// Test key 2 typed in and a one-key wallet made from it; returns the
/// wallet's index.
fn one_key(app: &mut Faraday) -> usize {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().fingerprint;
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(S::Make));
    app.session.wallets.len() - 1
}

/// Presses `action`, which the screen on show must offer somewhere down
/// its column.
fn press_offered(app: &mut Faraday, action: Action) {
    // Make the checklist is on The plan card (`docs/BACKUP-FLOW.md`
    // §15.1): a person opens it first.
    if action == Action::BChecklist && app.backup().is_some_and(|b| b.q != Some(qstep::PLAN)) {
        app.press(Action::BQ(qstep::PLAN));
    }
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        if app.offers(action) {
            app.press(action);
            return;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    panic!("{action:?} is not offered");
}

fn done(app: &Faraday, item: Item) -> bool {
    let c = testkit::compute();
    app.backup_item_done_now(&c, item)
}

#[test]
fn the_checklist_marks_each_item_done_by_what_it_does() {
    let c = faraday_core::testkit::compute();
    let mut app = device(vec![vault_file()]);
    let w = one_key(&mut app);
    unlock(&mut app);
    app.press(Action::Backup(w));
    assert_eq!(app.backup().unwrap().q, Some(qstep::BOARD));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    press_offered(&mut app, Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    assert_eq!(app.backup().unwrap().stage, BStage::Checklist);
    assert_eq!(
        app.backup_items_now(&c),
        vec![
            bstep::BLANK,
            bstep::COPY,
            bstep::VAULT,
            bstep::SHEETS,
            bstep::ENVELOPE,
            bstep::STICKS
        ],
        "only what paper and a vault need, and the writes"
    );
    // The sheet is made For the stick with the checklist: done with no
    // press. The template is printed from Faraday Starter, and done by its
    // press. The rest wait for what they do.
    for item in [Item::Templates, Item::Sheets] {
        assert!(done(&app, item), "{item:?} not done with the checklist");
    }
    assert!(
        !app.outbox
            .iter()
            .any(|i| i.name.starts_with("blank-template")),
        "no template is made here"
    );
    for item in [Item::Copy(0), Item::Vault(0), Item::Envelopes, Item::Sticks] {
        assert!(!done(&app, item), "{item:?} done before anything");
    }
    // So the checklist opens on the copy, which is typed back.
    assert_eq!(app.backup().unwrap().open, Some(bstep::COPY));
    app.press(Action::BReveal);
    app.storage(StorageEvent::Cameras(Vec::new()));
    press_offered(&mut app, Action::BCheck);
    testkit::type_words(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    assert!(done(&app, Item::Copy(0)));
    assert!(!done(&app, Item::Vault(0)), "the copy is not the vault");
    // The seed and the wallet, into the vault, by the vault item's one
    // button.
    press_offered(&mut app, Action::BStep(bstep::VAULT));
    press_offered(&mut app, Action::BVaultSave(0));
    assert!(done(&app, Item::Vault(0)));
    // The sheet, For the stick since the checklist: its row offers
    // Remove, which undoes the item, and making it again redoes it.
    press_offered(&mut app, Action::BStep(bstep::SHEETS));
    press_offered(&mut app, Action::PublicRemove(w, 3));
    assert!(!done(&app, Item::Sheets));
    press_offered(&mut app, Action::BOut(3));
    assert!(done(&app, Item::Sheets));
    // The envelopes, by a press; then the writes, which are the
    // checklist's last item: one lock, then each stick.
    press_offered(&mut app, Action::BStep(bstep::ENVELOPE));
    press_offered(&mut app, Action::BNext(bstep::ENVELOPE));
    assert!(done(&app, Item::Envelopes));
    // The envelopes' press opened the writes, the last item.
    assert_eq!(app.backup().unwrap().open, Some(bstep::STICKS));
    press_offered(
        &mut app,
        Action::Flow(faraday_core::backup_flow::steps::FlowAction::WalletLock),
    );
}

#[test]
fn a_fresh_backup_starts_with_the_templates_and_envelopes_not_done() {
    let c = testkit::compute();
    let mut app = device(Vec::new());
    let w = one_key(&mut app);
    app.press(Action::Backup(w));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    app.press(Action::BChecklist);
    assert!(!app.backup_item_done_now(&c, Item::Templates));
    assert!(!app.backup_item_done_now(&c, Item::Envelopes));
}

/// What a lock does to the backup under way: it is gone, and opened again
/// from the wallet, which an unlock loads back.
fn reopened(app: &mut Faraday, w: usize) {
    faraday_core::testkit::backup_drop(app);
    app.press(Action::Backup(w));
    app.press(Action::BChecklist);
}

#[test]
fn the_descriptor_shown_as_a_code_stays_done_when_the_backup_is_opened_again() {
    let c = testkit::compute();
    let mut app = device(Vec::new());
    let w = one_key(&mut app);
    app.press(Action::Backup(w));
    faraday_core::testkit::backup_as(&mut app, plan::Preset::PaperVaultSoftware);
    app.press(Action::BAnswer(
        qrow::SOFTWARE,
        plan::software::SPARROW as u8,
    ));
    app.press(Action::BChecklist);
    assert!(app.backup_items_now(&c).contains(&bstep::SHOW));
    assert!(!done(&app, Item::ShowDescriptor));
    app.press(Action::QrWallet(w));
    assert!(done(&app, Item::ShowDescriptor));
    reopened(&mut app, w);
    assert!(done(&app, Item::ShowDescriptor), "forgotten when reopened");
}

#[test]
fn a_passphrase_typed_back_stays_checked_when_the_backup_is_opened_again() {
    let c = testkit::compute();
    let mut app = device(Vec::new());
    let m = faraday_bip::bip39::Mnemonic::parse(
        &c,
        faraday_bip::bip39::Language::English,
        &testkit::test_words(testkit::TEST_SEEDS[0].0),
    )
    .unwrap();
    app.session
        .keys
        .add_mnemonic(&c, &m, "lantern-orchid-41", "Key", None)
        .unwrap();
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(0))
        .unwrap();
    use faraday_core::testkit::KindText;
    let w = app
        .session
        .add_wallet(
            &c,
            "Spending",
            &format!("wpkh({text}/<0;1>/*)"),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.press(Action::Backup(w));
    faraday_core::testkit::backup_as(&mut app, plan::Preset::Paper);
    no_words(&mut app);
    put(&mut app, Thing::Pass(0), Zone::Paper(0));
    app.press(Action::BChecklist);
    assert!(
        app.backup_items_now(&c).contains(&bstep::PASSPHRASE),
        "{:?}",
        app.backup_items_now(&c)
    );
    assert!(!done(&app, Item::Passphrase(0)));
    assert_eq!(app.backup().unwrap().open, Some(bstep::PASSPHRASE));
    app.press(Action::BCheck);
    type_text(&mut app, "lantern-orchid-41");
    app.event(Event::Key(Key::Enter));
    assert!(
        done(&app, Item::Passphrase(0)),
        "the passphrase did not match"
    );
    reopened(&mut app, w);
    assert!(done(&app, Item::Passphrase(0)), "forgotten when reopened");
}

#[test]
fn making_the_checklist_with_sparrow_chosen_puts_the_wallet_file_for_the_stick() {
    let c = faraday_core::testkit::compute();
    let mut app = device(Vec::new());
    let w = one_key(&mut app);
    app.press(Action::Backup(w));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVaultSoftware);
    app.press(Action::BAnswer(
        qrow::SOFTWARE,
        plan::software::SPARROW as u8,
    ));
    // As a file: the text form beside the picture.
    app.press(Action::BAnswer(qrow::FORM, plan::form::TEXT as u8));
    assert!(
        app.outbox.is_empty(),
        "nothing is made before the checklist"
    );
    press_offered(&mut app, Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    let name = format!(
        "{}-wallet.json",
        faraday_core::file_stem(&app.session.wallets[w].name)
    );
    assert!(
        app.outbox.iter().any(|i| i.name == name),
        "{name} is not For the stick"
    );
    assert!(done(&app, Item::PublicFiles));
}

#[test]
fn a_place_name_is_kept_in_the_vault_and_in_no_file() {
    let c = faraday_core::testkit::compute();
    let mut app = device(vec![vault_file()]);
    let w = one_key(&mut app);
    unlock(&mut app);
    app.press(Action::Backup(w));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVaultSoftware);
    // Start from goes on to the board itself; a place's name is typed
    // on its card there (`docs/BACKUP-FLOW.md` §17.12 item 2).
    if app.backup().unwrap().q != Some(qstep::BOARD) {
        press_offered(&mut app, Action::BQ(qstep::BOARD));
    }
    press_offered(&mut app, Action::BName(0));
    type_text(&mut app, NAME);
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.place_name(0), NAME);
    assert_eq!(app.place_name(1), "Place 2");
    // Every file the plan can make: the description as text and as a QR
    // picture on a stick at Place 1, the seed's words file beside them.
    put(&mut app, Thing::Whole, Zone::NewStick(0));
    let s = files_stick(&app, 0).expect("a stick of files at Place 1");
    form(&mut app, Zone::Files(0, s), Thing::Whole, 1);
    app.press(Action::BAnswer(
        qrow::FORM,
        faraday_core::plan::form::TEXT as u8,
    ));
    put(&mut app, Thing::Seed(0), Zone::Files(0, s));
    press_offered(&mut app, Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    let plans: Vec<_> = app
        .vaults
        .store()
        .contents_for_tests(app.vaults.open[0].id)
        .of(kind::PLAN)
        .map(|(_, r)| r.clone())
        .collect();
    assert_eq!(plans.len(), 1, "one plan for the wallet");
    assert_eq!(plans[0].text(field::PLAN_PLACE), Some(NAME));
    for what in [0u8, 1, 3, 5, 7, 8] {
        app.press(Action::PublicOut(w, what));
    }
    app.press(Action::QrWallet(w));
    app.press(Action::Cancel);
    app.press(Action::BFile);
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    assert!(app.outbox.len() >= 6, "{:?}", app.outbox.len());
    for item in &app.outbox {
        assert!(
            !item.bytes.windows(NAME.len()).any(|b| b == NAME.as_bytes()),
            "{} holds the place's name",
            item.name
        );
    }
    // Made again, the plan is replaced, not added to: Vault 1's stick at
    // Place 1 too.
    app.press(Action::BPlan);
    put(&mut app, Thing::Vault(0), Zone::Card(0));
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    assert_eq!(
        app.vaults
            .store()
            .contents_for_tests(app.vaults.open[0].id)
            .of(kind::PLAN)
            .count(),
        1
    );
    // The next backup of the wallet starts from it.
    let answers = app.backup().unwrap().answers.clone();
    app.press(Action::Nav(Screen::Wallets));
    app.press(Action::Backup(w));
    let b = app.backup().unwrap();
    assert_eq!(b.answers, answers);
    assert_eq!(b.q, None, "not the presets");
    assert_eq!(app.place_name(0), NAME);
}

#[test]
fn without_a_vault_places_have_no_names() {
    let mut app = device(Vec::new());
    let w = one_key(&mut app);
    app.press(Action::Backup(w));
    app.press(Action::BQ(qstep::BOARD));
    let _ = app.frame();
    assert!(!app.offers(Action::BName(0)));
    app.press(Action::BName(0));
    type_text(&mut app, NAME);
    assert_eq!(app.place_name(0), "Place 1");
}

#[test]
fn the_blank_template_has_a_line_per_place_and_no_name() {
    let c = faraday_core::testkit::compute();
    let mut app = device(vec![vault_file()]);
    let w = one_key(&mut app);
    unlock(&mut app);
    app.press(Action::Backup(w));
    // Paper only over two places, as a plan saved before one place per
    // key (`docs/BACKUP-FLOW.md` §17.19): the seed's words at each.
    crate::common::preset_two_places(&mut app, 1);
    assert_eq!(app.backup().unwrap().answers.places, 2);
    app.press(Action::BName(1));
    type_text(&mut app, NAME);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    // The blank template is printed from Faraday Starter: nothing is made
    // here, and no place's name is on any file.
    app.press(Action::BOut(0));
    assert!(
        !app.outbox
            .iter()
            .any(|i| i.name.starts_with("blank-template")),
        "the checklist makes no template"
    );
    app.press(Action::BStep(bstep::BLANK));
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t.starts_with("Print 2 from Faraday Starter: Blank templates, 24 words")),
        "the Templates item says to print them from Faraday Starter: {:?}",
        app.drawn_texts()
    );
    for item in &app.outbox {
        assert!(
            !item.bytes.windows(NAME.len()).any(|b| b == NAME.as_bytes()),
            "{} holds the place's name",
            item.name
        );
    }
}

/// Test keys 1 and 2 typed in and the 2-of-3 over the three test keys
/// loaded: two of its seeds here, the third a cosigner's. Returns the
/// wallet's index.
fn two_of_three_two_here(app: &mut Faraday) -> usize {
    let c = faraday_core::testkit::compute();
    for i in 0..8u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x40 + i; 32],
        )));
    }
    for k in 0..2 {
        app.press(Action::Entry(None));
        type_text(app, &testkit::test_words(testkit::TEST_SEEDS[k].0));
        app.press(Action::EntryAdd);
    }
    assert_eq!(app.session.keys.len(), 2);
    app.session
        .add_wallet(
            &c,
            "Savings",
            &testkit::savings(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap()
}

/// Create a vault, opened from the backup: a passphrase typed twice, the
/// vault made, unlocked, and back to the backup.
fn make_the_vault(app: &mut Faraday) {
    assert_eq!(app.screen, Screen::CreateVault);
    let before = app.vaults.open.len();
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(app, "test phrase");
    }
    app.press(Action::Vault(V::CGo));
    for t in 1..60u64 {
        if app.screen == Screen::Unlock {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    type_text(app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    for t in 60..120u64 {
        if app.vaults.open.len() > before && app.screen == Screen::Backup {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.screen, Screen::Backup, "{:?}", app.vaults.unlock_error);
}

#[test]
fn each_vault_is_its_own_item_and_the_second_does_not_take_the_vault_the_first_filled() {
    let c = faraday_core::testkit::compute();
    let mut app = device(Vec::new());
    let w = two_of_three_two_here(&mut app);
    app.press(Action::Backup(w));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    // No paper words, so the vaults come first.
    no_words(&mut app);
    press_offered(&mut app, Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    let items = app.backup_items_now(&c);
    assert!(
        items.contains(&bstep::VAULT) && items.contains(&(bstep::VAULT + 1)),
        "two vault items: {items:?}"
    );
    assert!(
        !items.contains(&(bstep::VAULT + 2)),
        "no vault for the cosigner's seed"
    );
    // Vault 1, open first: made, its seed and the wallet saved into it.
    assert_eq!(app.backup().unwrap().open, Some(bstep::VAULT));
    press_offered(&mut app, Action::Vault(V::CreateFrom(Screen::Backup)));
    make_the_vault(&mut app);
    assert_eq!(app.backup().unwrap().open, Some(bstep::VAULT));
    press_offered(&mut app, Action::BVaultSave(0));
    assert!(done(&app, Item::Vault(0)));
    assert!(!done(&app, Item::Vault(1)));
    // Vault 2's item: the open vault holds Vault 1's seed, so it offers
    // no save, only to lock it and make a new one.
    press_offered(&mut app, Action::BStep(bstep::VAULT + 1));
    let mut found = false;
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        assert!(
            !app.offers(Action::BVaultSave(1)),
            "Vault 2 offered into Vault 1's vault"
        );
        if app.offers(Action::BNewVault(1)) {
            found = true;
            break;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    assert!(found, "Lock it and make a new vault is not offered");
    app.press(Action::BNewVault(1));
    assert_eq!(app.vaults.open.len(), 0, "the first vault is locked");
    make_the_vault(&mut app);
    assert_eq!(app.backup().unwrap().open, Some(bstep::VAULT + 1));
    press_offered(&mut app, Action::BVaultSave(1));
    assert!(done(&app, Item::Vault(1)));
    assert!(
        done(&app, Item::Vault(0)),
        "the first vault, locked, still did its item"
    );
    assert_eq!(
        app.outbox
            .iter()
            .filter(|i| i.name.ends_with(".ofv"))
            .count(),
        2,
        "two vault files for the sticks"
    );
}

/// A drawn string and its box in pixels.
type TextBox = (String, (i32, i32, i32, i32));

/// Whether the card titled `title` shows a tick: its number, drawn left
/// of its title, is gone.
fn ticked(boxes: &[TextBox], title: &str, number: usize) -> bool {
    let (_, (x, y, _, _)) = boxes
        .iter()
        .find(|(t, _)| t == title)
        .unwrap_or_else(|| panic!("no card titled {title:?} in {boxes:?}"));
    !boxes
        .iter()
        .any(|(t, (bx, by, _, _))| *t == number.to_string() && bx < x && (by - y).abs() < 24)
}

/// The copy by hand of the wallet's seed `i`, checked by its words typed
/// back.
fn check_copy(app: &mut Faraday, w: usize, i: usize) {
    let k = app.backup_seed_list_now(w)[i].1.expect("the seed is here");
    let words = app
        .session
        .keys
        .reveal(k, faraday_keys::Reveal::Words)
        .to_string();
    let item = bstep::COPY + i as u8;
    if app.backup().unwrap().open != Some(item) {
        press_offered(app, Action::BStep(item));
    }
    app.press(Action::BReveal);
    app.storage(StorageEvent::Cameras(Vec::new()));
    press_offered(app, Action::BCheck);
    testkit::type_words(app, &words);
    assert!(done(app, Item::Copy(i)), "copy {i} did not check");
}

#[test]
fn through_a_2_of_3_checklist_the_cards_tick_what_is_done_and_the_rest_wait_for_the_first_not_done()
{
    let c = faraday_core::testkit::compute();
    let mut app = device(Vec::new());
    // Tall enough that every card is drawn.
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
    let w = two_of_three_two_here(&mut app);
    app.press(Action::Backup(w));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    press_offered(&mut app, Action::BChecklist);
    assert_eq!(
        app.backup_items_now(&c),
        vec![
            bstep::BLANK,
            bstep::COPY,
            bstep::COPY + 1,
            bstep::SHEETS,
            bstep::ENVELOPE,
            bstep::STICKS
        ]
    );
    let seed = |i: usize| app.backup_seed_list_now(w)[i].0.clone();
    let titles = [
        "Print 2 blank templates".to_string(),
        format!("Copy seed {} by hand", seed(0)),
        format!("Copy seed {} by hand", seed(1)),
        "The wallet sheet".to_string(),
        "One envelope per place".to_string(),
        "Write to sticks".to_string(),
    ];
    // After each press: the cards ticked, and the first one not done,
    // which every card after it not done names.
    let check = |app: &mut Faraday, ticks: [bool; 6], waiting: Option<&str>| {
        app.settle();
        let boxes = app.drawn_text_boxes();
        for (i, title) in titles.iter().enumerate() {
            assert_eq!(ticked(&boxes, title, i + 1), ticks[i], "{title}: {boxes:?}");
        }
        let after: Vec<&str> = boxes
            .iter()
            .filter(|(t, _)| t.starts_with("After: "))
            .map(|(t, _)| t.as_str())
            .collect();
        match waiting {
            Some(first) => {
                let behind = ticks
                    .iter()
                    .enumerate()
                    .filter(|&(i, done)| !done && titles[i] != first)
                    .count();
                assert_eq!(after.len(), behind, "{after:?}");
                assert!(
                    after.iter().all(|t| *t == format!("After: {first}")),
                    "{after:?}"
                );
            }
            None => assert!(after.is_empty(), "{after:?}"),
        }
    };
    // The sheet is made with the checklist; nothing else is.
    check(
        &mut app,
        [false, false, false, true, false, false],
        Some(&titles[0]),
    );
    app.press(Action::BNext(bstep::BLANK));
    check(
        &mut app,
        [true, false, false, true, false, false],
        Some(&titles[1]),
    );
    check_copy(&mut app, w, 0);
    check(
        &mut app,
        [true, true, false, true, false, false],
        Some(&titles[2]),
    );
    check_copy(&mut app, w, 1);
    check(
        &mut app,
        [true, true, true, true, false, false],
        Some(&titles[4]),
    );
    press_offered(&mut app, Action::BStep(bstep::ENVELOPE));
    press_offered(&mut app, Action::BNext(bstep::ENVELOPE));
    check(
        &mut app,
        [true, true, true, true, true, false],
        Some(&titles[5]),
    );
}

/// Whether the backup's column offers `action` anywhere down it: scrolled
/// from the top to the end, then back to the top.
fn offered_anywhere(app: &mut Faraday, action: Action) -> bool {
    let mut seen = false;
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        seen |= app.offers(action);
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    for _ in 0..20 {
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: -300,
        });
    }
    app.settle();
    let _ = app.frame();
    seen
}

/// Opens the checklist's public files item afresh: closed, then opened.
fn reopen_public(app: &mut Faraday) {
    if app.backup().and_then(|b| b.open) == Some(bstep::PUBLIC) {
        app.press(Action::BStep(bstep::PUBLIC));
    }
    app.press(Action::BStep(bstep::PUBLIC));
    assert_eq!(app.backup().and_then(|b| b.open), Some(bstep::PUBLIC));
}

#[test]
fn public_files_under_more_options_show_only_once_it_is_pressed_and_none_is_made_unasked() {
    let c = faraday_core::testkit::compute();
    let mut app = device(vec![vault_file()]);
    let w = one_key(&mut app);
    unlock(&mut app);
    app.press(Action::Backup(w));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVaultSoftware);
    app.press(Action::BAnswer(
        qrow::SOFTWARE,
        plan::software::SPARROW as u8,
    ));
    app.press(Action::BAnswer(qrow::FORM, plan::form::TEXT as u8));
    press_offered(&mut app, Action::BChecklist);
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    // The items before the public files: the copy typed back, the vault.
    app.press(Action::BReveal);
    app.storage(StorageEvent::Cameras(Vec::new()));
    press_offered(&mut app, Action::BCheck);
    testkit::type_words(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    press_offered(&mut app, Action::BStep(bstep::VAULT));
    press_offered(&mut app, Action::BVaultSave(0));
    // Making the checklist makes no file that is under More options.
    let stem = faraday_core::file_stem(&app.session.wallets[w].name);
    let electrum = format!("{stem}-electrum.json");
    let core = format!("{stem}-bitcoin-core-before-29.json");
    assert!(
        !app.outbox
            .iter()
            .any(|i| i.name == electrum || i.name == core || i.name.ends_with("-wasabi.json")),
        "{:?}",
        app.outbox.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
    reopen_public(&mut app);
    // A fresh item: the button, none of the files under it.
    assert!(offered_anywhere(&mut app, Action::PublicMore(w)));
    for what in [9, 10] {
        assert!(
            !offered_anywhere(&mut app, Action::PublicOut(w, what)),
            "{what}"
        );
    }
    app.press(Action::PublicMore(w));
    assert!(!offered_anywhere(&mut app, Action::PublicMore(w)));
    for what in [9, 10] {
        assert!(
            offered_anywhere(&mut app, Action::PublicOut(w, what)),
            "{what}"
        );
    }
    // Left and opened again with none of them made: closed again.
    reopen_public(&mut app);
    assert!(offered_anywhere(&mut app, Action::PublicMore(w)));
    // With one made, it opens open.
    app.press(Action::PublicMore(w));
    app.press(Action::PublicOut(w, 10));
    assert!(app.outbox.iter().any(|i| i.name == electrum));
    reopen_public(&mut app);
    assert!(!offered_anywhere(&mut app, Action::PublicMore(w)));
    assert!(offered_anywhere(&mut app, Action::PublicOut(w, 9)));
}
