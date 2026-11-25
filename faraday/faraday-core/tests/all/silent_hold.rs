//! A silent payments address over a key made here waits for that key's
//! backup, as a receive address does (F31): the page, its card, its
//! codes and its DNS record say "Back up before you receive" in place of
//! the address, and the backup done shows them again.

use crate::common::RecordsOf as _;
use crate::common::{Disk, Shell, keep_time, let_seed_files_out, process};
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::edit::Here;
use faraday_core::plan::{At, What, form};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Code, Faraday, Screen, plan, qrow, testkit};
use faraday_shell_api::{App, Event, Key};

const WARNING: &str = "Back up before you receive";

/// The first test key, loaded and made here, with its address.
fn made_here() -> (Faraday, String) {
    made_here_in(testkit::started())
}

fn made_here_in(mut app: Faraday) -> (Faraday, String) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    testkit::keep_published(&mut app);
    let c = testkit::compute();
    app.press(Action::Silent);
    // A screen wraps a long address: its start is what is looked for.
    let address = app.silent_address(&c).unwrap().to_string();
    let fp = app.session.keys[0].fingerprint;
    testkit::mark_made_here(&mut app, fp);
    (app, address)
}

fn drawn(app: &mut Faraday) -> String {
    app.drawn_texts().join(" ")
}

fn type_dns(app: &mut Faraday) {
    for (here, text) in [
        (Here::SilentName, "matt"),
        (Here::SilentDomain, "example.com"),
    ] {
        app.press(Action::TypeHere(here));
        for c in text.chars() {
            app.event(Event::Key(Key::Char(c)));
        }
    }
}

#[test]
fn the_page_shows_the_warning_for_the_address_and_its_dns_record_while_the_key_waits() {
    let (mut app, address) = made_here();
    type_dns(&mut app);
    let text = drawn(&mut app);
    assert!(text.contains(WARNING), "{text}");
    assert!(!text.contains(&address[..24]), "{text}");
    assert!(!text.contains("matt.user._bitcoin-payment"), "{text}");
    assert!(!text.contains("Show as QR"), "{text}");
    assert!(!text.contains("As a bitcoin: link"), "{text}");
    assert!(!text.contains("DNS record as QR"), "{text}");
    // After the backup the record is there again.
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    let text = drawn(&mut app);
    assert!(text.contains("matt.user._bitcoin-payment"), "{text}");
    assert!(text.contains("DNS record as QR"), "{text}");
}

#[test]
fn no_code_of_the_address_or_its_dns_record_is_made_while_the_key_waits_and_the_record_is_warned() {
    let (mut app, _) = made_here();
    type_dns(&mut app);
    let c = testkit::compute();
    for code in [Code::Silent(false), Code::Silent(true), Code::SilentDns] {
        assert!(app.code_view(&c, code).is_err(), "{code:?}");
    }
    let view = app.code_view(&c, Code::SilentRecord).unwrap();
    assert_eq!(view.label[0].to_string(), WARNING);
}

#[test]
fn the_wallet_card_and_its_descriptor_code_are_held_with_the_key() {
    let (mut app, address) = made_here();
    app.press(Action::SAddWallet);
    app.press(Action::Nav(Screen::Wallets));
    let text = drawn(&mut app);
    assert!(text.contains(WARNING), "{text}");
    assert!(!text.contains(&address[..24]), "{text}");
    let c = testkit::compute();
    assert!(app.receive_held(&c, &app.session.wallets[0]));
    // A silent wallet's descriptor is its address: no code of it either,
    // nor its picture among the public files.
    assert!(app.code_view(&c, Code::Descriptor(0)).is_err());
    app.press(Action::PublicOut(0, 8));
    assert!(
        !app.outbox
            .iter()
            .any(|i| i.name.ends_with("-descriptor.png")),
        "{:?}",
        app.outbox.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
    // The press says so, and no button for the code is drawn.
    assert!(app.toast_text().is_some_and(|t| t.contains("No address")));
    assert!(!drawn(&mut app).contains("Show wallet QR"));
    // After the backup both are made.
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    assert!(app.code_view(&c, Code::Descriptor(0)).is_ok());
    app.press(Action::PublicOut(0, 8));
    assert!(
        app.outbox
            .iter()
            .any(|i| i.name.ends_with("-descriptor.png")),
        "{:?}",
        app.outbox.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
}

#[test]
fn a_held_silent_wallets_backup_does_not_say_its_key_signs_once_done_and_records_no_hold() {
    let (mut app, _) = made_here();
    app.press(Action::SAddWallet);
    let c = testkit::compute();
    // The plan, Who signs and the plan record read this: the wallet's
    // backup places no seed, so it releases nothing.
    assert!(app.wallet_held(&c, 0).is_empty());
    assert!(app.receive_held(&c, &app.session.wallets[0]));
}

#[test]
fn the_silent_pages_choices_row_withholds_the_address_too() {
    let (mut app, address) = made_here();
    app.press(Action::SAddWallet);
    app.press(Action::SWallet(0));
    let text = drawn(&mut app);
    // The row keeps the start and end of an address: neither is shown.
    assert!(!text.contains(&address[address.len() - 4..]), "{text}");
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    app.press(Action::SWallet(0));
    let text = drawn(&mut app);
    assert!(text.contains(&address[address.len() - 4..]), "{text}");
}

#[test]
fn the_wallets_badge_counts_a_held_silent_wallet_until_its_backup_is_done() {
    let (mut app, _) = made_here();
    app.press(Action::SAddWallet);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    let badge = |app: &mut Faraday| app.drawn_texts().iter().any(|t| t == "1");
    assert!(badge(&mut app));
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    assert!(!badge(&mut app));
}

/// The key made here, with a record of another fingerprint added first:
/// the page is on that record, whose key is not loaded.
fn other_record_first() -> (Faraday, faraday_bip::silent_wallet::SilentWallet) {
    let (mut app, _) = made_here();
    let c = testkit::compute();
    let text = app.silent_record(&c).unwrap();
    let mut other = faraday_bip::silent_wallet::SilentWallet::parse(&text).unwrap();
    other.fingerprint = faraday_bip::keys::Fingerprint([0; 4]);
    // Another key's points, not the loaded key's: a record that names the
    // loaded key's own points under another fingerprint is refused.
    other.scan = other.spend;
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            "silent-0.txt".to_string(),
            other.to_text().into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    app.press(Action::Silent);
    app.press(Action::SAddWallet);
    // The page is on the other record.
    app.press(Action::SWallet(0));
    (app, other)
}

#[test]
fn a_record_of_another_fingerprint_added_first_does_not_unhold_the_keys_own_page() {
    let (mut app, _) = other_record_first();
    let c = testkit::compute();
    // The page is on the other record, whose key is not loaded: nothing
    // waits for it.
    assert!(!app.silent_held(&c));
    let text = drawn(&mut app);
    assert!(text.contains("Its key is not loaded"), "{text}");
    assert!(!text.contains(WARNING), "{text}");
    // The key's own record, second, stays held.
    app.press(Action::SWallet(1));
    assert!(app.silent_held(&c));
    let text = drawn(&mut app);
    assert!(text.contains(WARNING), "{text}");
}

#[test]
fn a_record_naming_the_waiting_keys_points_under_another_fingerprint_is_refused() {
    let (mut app, _) = made_here();
    let c = testkit::compute();
    let text = app.silent_record(&c).unwrap();
    let mut other = faraday_bip::silent_wallet::SilentWallet::parse(&text).unwrap();
    other.fingerprint = faraday_bip::keys::Fingerprint([0; 4]);
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            "silent-0.txt".to_string(),
            other.to_text().into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut app, 0);
    assert!(app.session.wallets.is_empty());
    let toast = app.toast_text().unwrap_or_default();
    assert!(toast.contains("its record names key 00000000"), "{toast}");
}

#[test]
fn the_other_records_page_offers_its_code_while_another_key_waits() {
    let (mut app, _) = other_record_first();
    let text = drawn(&mut app);
    assert!(text.contains("Its key is not loaded"), "{text}");
    assert!(text.contains("Show as QR"), "{text}");
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    let text = drawn(&mut app);
    assert!(text.contains("Its key is not loaded"), "{text}");
    assert!(text.contains("Show as QR"), "{text}");
}

#[test]
fn the_other_records_page_on_the_waiting_key_draws_no_code_button_and_is_held() {
    let (mut app, _) = other_record_first();
    let c = testkit::compute();
    let waiting = app.session.keys[0].fingerprint.0;
    app.press(Action::SStep(faraday_core::silent::sstep::KEY));
    app.press(Action::SKey(waiting));
    // The page is still on the other record, whose key is not loaded, and
    // its Key step is on the key that waits.
    assert!(app.silent_held(&c));
    let text = drawn(&mut app);
    assert!(text.contains("Its key is not loaded"), "{text}");
    assert!(!text.contains("Show as QR"), "{text}");
}

#[test]
fn a_label_is_not_stepped_to_while_the_address_waits_for_its_backup() {
    let (mut app, _) = made_here();
    let c = testkit::compute();
    let labels = |app: &Faraday| {
        let text = app.silent_record(&c).unwrap();
        faraday_bip::silent_wallet::SilentWallet::parse(&text)
            .unwrap()
            .labels
    };
    let before = labels(&app);
    app.press(Action::SLabel(1));
    assert_eq!(labels(&app), before);
    // After the backup the same press steps to a label.
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    app.press(Action::SLabel(1));
    assert_ne!(labels(&app), before);
}

#[test]
fn the_vault_item_of_a_held_silent_wallet_shows_the_warning_for_its_record() {
    let (mut app, address) = made_here();
    app.press(Action::SAddWallet);
    crate::common::open_plain_vault(&mut app);
    app.press(Action::Vault(V::SaveWallet(0)));
    let shown = crate::common::vault_wallet_item_text(&mut app);
    assert!(shown.contains(WARNING), "{shown}");
    assert!(!shown.contains(&address[..24]), "{shown}");
}

#[test]
fn the_backup_done_shows_the_address_again_and_a_key_not_made_here_never_held_it() {
    let (mut app, address) = made_here();
    app.press(Action::SAddWallet);
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    app.press(Action::SWallet(0));
    let text = drawn(&mut app);
    assert!(text.contains(&address[..24]), "{text}");
    assert!(!text.contains(WARNING), "{text}");
    assert!(text.contains("Show as QR"), "{text}");
    let c = testkit::compute();
    assert!(app.code_view(&c, Code::Silent(false)).is_ok());
    assert!(app.code_view(&c, Code::Descriptor(0)).is_ok());
    assert!(!app.receive_held(&c, &app.session.wallets[0]));
}

#[test]
fn a_key_not_made_here_never_held_its_address() {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    testkit::keep_published(&mut app);
    let c = testkit::compute();
    app.press(Action::Silent);
    let address = app.silent_address(&c).unwrap().to_string();
    app.press(Action::SAddWallet);
    assert!(!app.silent_held(&c));
    assert!(!app.receive_held(&c, &app.session.wallets[0]));
    app.press(Action::SWallet(0));
    let text = drawn(&mut app);
    assert!(text.contains(&address[..24]), "{text}");
    assert!(!text.contains(WARNING), "{text}");
    assert!(app.code_view(&c, Code::Descriptor(0)).is_ok());
}

/// Backup of the held silent wallet, the text form chosen beside the
/// picture, its checklist made.
fn held_backup_checklist() -> (Faraday, faraday_crypto::work::Compute) {
    let (mut app, _) = made_here();
    app.press(Action::SAddWallet);
    app.press(Action::Backup(0));
    testkit::backup_as(&mut app, plan::Preset::PaperVaultSoftware);
    app.press(Action::BAnswer(
        qrow::SOFTWARE,
        plan::software::SPARROW as u8,
    ));
    app.press(Action::BAnswer(qrow::FORM, plan::form::TEXT as u8));
    app.press(Action::BChecklist);
    (app, testkit::compute())
}

#[test]
fn the_backup_of_a_held_silent_wallet_with_the_picture_and_the_text_chosen_finishes_its_public_files()
 {
    let (mut app, c) = held_backup_checklist();
    let names = testkit::backup_public_names(&app, &c);
    assert!(
        names.iter().any(|n| n.ends_with("-descriptor.txt")),
        "{names:?}"
    );
    assert!(!names.iter().any(|n| n.ends_with(".png")), "{names:?}");
    assert!(app.backup_items_now(&c).contains(&plan::Item::PublicFiles));
    assert!(app.backup_item_done_now(&c, plan::Item::PublicFiles));
    // After the backup the picture is named again.
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    let names = testkit::backup_public_names(&app, &c);
    assert!(
        names.iter().any(|n| n.ends_with("-descriptor.png")),
        "{names:?}"
    );
}

#[test]
fn a_descriptor_picture_kept_at_a_place_is_the_text_file_while_the_address_is_held() {
    let (mut app, c) = held_backup_checklist();
    let picture = plan::What::WalletFile(plan::form::QR);
    let names = testkit::stick_file_names(&app, &c, 0, picture);
    assert_eq!(names.len(), 1, "{names:?}");
    assert!(names[0].ends_with("-descriptor.txt"), "{names:?}");
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    let names = testkit::stick_file_names(&app, &c, 0, picture);
    assert!(names[0].ends_with("-descriptor.png"), "{names:?}");
}

/// One stick written as the flow asks: confirmed, held where it must be.
fn write_stick(shell: &mut Shell, app: &mut Faraday, d: &str) {
    shell.plug(app, d);
    for _ in 0..3 {
        let r = app.flow().and_then(|f| f.live.reading.as_ref());
        let Some(r) = r else { break };
        if r.problem.is_some() || r.bound {
            break;
        }
        if r.hold {
            shell.flow(app, F::Hold);
        } else {
            shell.flow(app, F::Tap);
        }
    }
    shell.pull(app, d);
}

/// Makes and saves into each vault the plan lists.
fn save_plan_vaults(app: &mut Faraday) {
    let c = testkit::compute();
    let vaults: Vec<usize> = app
        .backup_items_now(&c)
        .into_iter()
        .filter_map(|n| match n {
            plan::Item::Vault(v) => Some(v),
            _ => None,
        })
        .collect();
    for v in vaults {
        if app.vaults().open.len() <= v {
            crate::common::make_plan_vault(app, v, "first vault phrase");
        }
        app.press(Action::BVaultSave(v as u8));
    }
}

/// A held silent wallet's backup with the plan edited by `edit`, Lock and
/// write, and each stick written; the names of the files on the sticks.
fn lock_and_write_held(
    text: bool,
    edit: impl FnOnce(Vec<plan::Spot>) -> Vec<plan::Spot>,
) -> Vec<String> {
    lock_and_write_held_in(plan::Preset::Paper, text, None, edit)
}

/// [`lock_and_write_held`] for `preset`, the watch-only software `only` (as
/// the only one ticked) where asked. Nothing is left For the stick once
/// the sticks are written.
fn lock_and_write_held_in(
    preset: plan::Preset,
    text: bool,
    only: Option<usize>,
    edit: impl FnOnce(Vec<plan::Spot>) -> Vec<plan::Spot>,
) -> Vec<String> {
    let (mut shell, mut app) = lock_held_in(preset, text, only, edit);
    write_all_sticks(&mut shell, &mut app)
}

/// The flow of [`lock_and_write_held_in`] as it stands after the lock's
/// restart, nothing written yet.
fn lock_held_in(
    preset: plan::Preset,
    text: bool,
    only: Option<usize>,
    edit: impl FnOnce(Vec<plan::Spot>) -> Vec<plan::Spot>,
) -> (Shell, Faraday) {
    let mut shell = Shell::light();
    for k in 1..=3u32 {
        let mut d = Disk::empty(&format!("S{k}"), "KEYSTICK");
        d.serial = Some(k);
        shell.add(d);
    }
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    let (mut app, _) = made_here_in(app);
    app.press(Action::SAddWallet);
    app.press(Action::Backup(0));
    testkit::backup_as(&mut app, preset);
    app.press(Action::BAnswer(
        qrow::SOFTWARE,
        plan::software::SPARROW as u8,
    ));
    if text {
        app.press(Action::BAnswer(qrow::FORM, plan::form::TEXT as u8));
    }
    if let Some(k) = only {
        testkit::change_backup_answers(&mut app, |a| {
            a.software = [false; 5];
            a.software[k] = true;
        });
    }
    let c = testkit::compute();
    let shape = app.plan_shape_now(app.backup().unwrap().wallet);
    testkit::change_backup_answers(&mut app, |a| {
        let m = plan::map(&c, &shape, a);
        a.set_map(edit(m));
    });
    app.press(Action::BChecklist);
    save_plan_vaults(&mut app);
    app.press(Action::BFilesMake);
    testkit::type_passphrases(&mut app);
    app.press(Action::BNext(plan::Item::Envelopes));
    shell.flow(&mut app, F::WalletLock);
    let_seed_files_out(&mut app);
    shell.pump(&mut app);
    assert!(
        app.restart_requested(),
        "locked: {:?} {:?} {:?} {:?}",
        app.backup().map(|b| b.stage),
        app.screen,
        app.backup_items_now(&c)
            .iter()
            .map(|i| (*i, app.backup_item_done_now(&c, *i)))
            .collect::<Vec<_>>(),
        app.flow().map(|f| f.step)
    );
    let app = shell.restart();
    (shell, app)
}

/// Each slot of the flow written; the names of the files on the sticks.
fn write_all_sticks(shell: &mut Shell, app: &mut Faraday) -> Vec<String> {
    let n = app.flow().unwrap().slots.len();
    for k in 0..n {
        write_stick(shell, app, &format!("S{}", k + 1));
    }
    assert_eq!(
        app.flow().unwrap().step,
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    assert!(
        app.outbox.is_empty(),
        "left For the stick: {:?}",
        app.outbox.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
    ["S1", "S2", "S3"]
        .iter()
        .flat_map(|d| shell.disk(d).files.iter().map(|f| f.0.clone()))
        .collect()
}

#[test]
fn a_stick_holding_only_the_descriptor_picture_is_written_and_the_backup_finishes_while_held() {
    let files = lock_and_write_held(true, |m| {
        plan::with_added(&m, At::Stick(0, 0), What::WalletFile(form::QR))
    });
    assert!(
        files.iter().any(|n| n.ends_with("-descriptor.txt")),
        "{files:?}"
    );
    assert!(!files.iter().any(|n| n.ends_with(".png")), "{files:?}");
}

#[test]
fn a_stick_holding_the_descriptor_picture_and_another_file_has_the_text_file_while_held() {
    // The software ticked is Sparrow alone, whose file is not the text
    // file: the place's own file is the only one that is.
    let files = lock_and_write_held_in(
        plan::Preset::Paper,
        true,
        Some(plan::software::SPARROW),
        |m| {
            let m = plan::with_added(&m, At::Stick(0, 0), What::SoftwareFile);
            plan::with_added(&m, At::Stick(0, 0), What::WalletFile(form::QR))
        },
    );
    assert!(
        files.iter().any(|n| n.ends_with("-descriptor.txt")),
        "{files:?}"
    );
    assert!(!files.iter().any(|n| n.ends_with(".png")), "{files:?}");
}

#[test]
fn a_held_silent_wallet_lists_the_text_file_in_place_of_the_descriptor_code() {
    let (mut app, c) = held_backup_checklist();
    let items = app.backup_items_now(&c);
    assert!(!items.contains(&plan::Item::ShowDescriptor), "{items:?}");
    assert!(items.contains(&plan::Item::PublicFiles), "{items:?}");
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    let items = app.backup_items_now(&c);
    assert!(items.contains(&plan::Item::ShowDescriptor), "{items:?}");
    assert!(!app.backup_item_done_now(&c, plan::Item::ShowDescriptor));
    app.press(Action::QrWallet(0));
    assert!(app.backup_item_done_now(&c, plan::Item::ShowDescriptor));
}

#[test]
fn the_exported_public_record_is_not_held() {
    let (mut app, _) = made_here();
    app.press(Action::SRecord);
    assert!(!app.outbox.is_empty());
}

/// The map with the sheet taken off the paper.
fn no_sheet(m: Vec<plan::Spot>) -> Vec<plan::Spot> {
    m.into_iter()
        .map(|mut s| {
            s.holds.retain(|(w, _)| *w != What::Sheet);
            s
        })
        .collect()
}

fn software(m: Vec<plan::Spot>) -> Vec<plan::Spot> {
    plan::with_added(&m, At::Software, What::Wallet)
}

fn has_text_file(files: &[String]) -> bool {
    files.iter().any(|n| n.ends_with("-descriptor.txt"))
}

#[test]
fn a_qr_form_plan_with_the_software_and_the_sheet_of_a_held_silent_wallet_writes_no_picture() {
    let files = lock_and_write_held(false, software);
    assert!(has_text_file(&files), "{files:?}");
    assert!(files.iter().all(|n| !n.ends_with(".png")), "{files:?}");
}

#[test]
fn a_qr_form_plan_with_the_software_of_a_held_silent_wallet_writes_the_text_file_without_the_sheet()
{
    let files = lock_and_write_held(false, |m| no_sheet(software(m)));
    assert!(has_text_file(&files), "{files:?}");
    assert!(files.iter().all(|n| !n.ends_with(".png")), "{files:?}");
}

#[test]
fn the_description_in_a_vault_with_the_software_on_and_no_sheet_is_written_as_the_text_file() {
    let files = lock_and_write_held_in(plan::Preset::PaperVault, false, None, |m| {
        no_sheet(software(plan::with_added(&m, At::Vault(0), What::Wallet)))
    });
    assert!(has_text_file(&files), "{files:?}");
    assert!(files.iter().all(|n| !n.ends_with(".png")), "{files:?}");
}

#[test]
fn a_picture_on_a_places_stick_and_sparrow_alone_write_both_files() {
    let files = lock_and_write_held_in(
        plan::Preset::Paper,
        false,
        Some(plan::software::SPARROW),
        |m| {
            let m = plan::with_added(&m, At::Stick(0, 0), What::WalletFile(form::QR));
            no_sheet(software(m))
        },
    );
    assert!(has_text_file(&files), "{files:?}");
    assert!(
        files.iter().any(|n| n.ends_with("-wallet.json")),
        "{files:?}"
    );
    assert!(files.iter().all(|n| !n.ends_with(".png")), "{files:?}");
}

#[test]
fn the_software_alone_writes_its_file_while_held() {
    let files = lock_and_write_held_in(
        plan::Preset::Paper,
        false,
        Some(plan::software::SPARROW),
        |m| no_sheet(software(m)),
    );
    assert!(
        files.iter().any(|n| n.ends_with("-wallet.json")),
        "{files:?}"
    );
}

#[test]
fn a_held_plans_record_says_the_text_file_where_the_map_keeps_the_picture() {
    let (mut app, c) = held_backup_checklist();
    let shape = app.plan_shape_now(0);
    // The picture is in the map, on a place's stick, and no sheet: the
    // record's holds lines are where the picture could be written.
    testkit::change_backup_answers(&mut app, |a| {
        a.form = [true, false];
        let m = plan::stored_map(&shape, a);
        let m = plan::with_added(&m, At::Stick(0, 0), What::WalletFile(form::QR));
        a.set_map(no_sheet(m));
    });
    save_plan_vaults(&mut app);
    let a = app.backup().unwrap().answers.clone();
    let m = plan::Mapped::of(&c, &shape, &a);
    assert!(plan::public_stick_on(&a, &m));
    let items = app.backup_items_now(&c);
    assert!(items.contains(&plan::Item::PublicFiles), "{items:?}");
    assert!(!items.contains(&plan::Item::ShowDescriptor), "{items:?}");
    // The record kept in the open vault: the text file, never the picture.
    let held = |app: &Faraday| -> Vec<String> {
        app.vaults()
            .store()
            .contents_for_tests(app.vaults().open[0].id)
            .of(faraday_vault::records::kind::PLAN)
            .flat_map(|(_, r)| {
                faraday_keys::vault::RecordView::of(r)
                    .public_all(faraday_vault::records::field::PLAN_HOLDS)
                    .map(|b| String::from_utf8_lossy(b).to_string())
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    let lines = held(&app).join("\n");
    // The record is there and names the descriptor, never as the picture.
    assert!(lines.contains("Descriptor · Text file"), "{lines}");
    assert!(!lines.contains("QR picture"), "{lines}");
    // The answer as chosen is kept: the picture is back after the release.
    assert!(app.backup().unwrap().answers.form[form::QR]);
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    let items = app.backup_items_now(&c);
    assert!(items.contains(&plan::Item::ShowDescriptor), "{items:?}");
}

#[test]
fn an_ordinary_wallet_keeps_the_picture_with_the_software_and_the_qr_form() {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    for ch in testkit::test_words(testkit::TEST_SEEDS[1].0).chars() {
        app.event(Event::Key(Key::Char(ch)));
    }
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().fingerprint;
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    testkit::backup_as(&mut app, plan::Preset::Paper);
    app.press(Action::BAnswer(
        qrow::SOFTWARE,
        plan::software::SPARROW as u8,
    ));
    testkit::change_backup_answers(&mut app, |a| a.form = [true, false]);
    let c = testkit::compute();
    let shape = app.plan_shape_now(w);
    testkit::change_backup_answers(&mut app, |a| {
        let m = plan::map(&c, &shape, a);
        a.set_map(software(m));
    });
    app.press(Action::BChecklist);
    assert!(!app.plan_shape_now(w).text_files);
    let items = app.backup_items_now(&c);
    assert!(items.contains(&plan::Item::ShowDescriptor), "{items:?}");
    let names = testkit::backup_public_names(&app, &c);
    assert!(names.iter().any(|n| n.ends_with(".png")), "{names:?}");
}

#[test]
fn the_shape_a_dated_line_is_built_under_does_not_depend_on_the_hold() {
    let (app, _c) = held_backup_checklist();
    assert!(app.plan_shape_now(0).text_files, "held");
    assert!(!testkit::plan_shape_made(&app, 0).text_files);
}

/// The flow after the lock's restart: a public stick of its own, no sheet,
/// the QR form and the software.
fn restarted_held_flow() -> (Shell, Faraday) {
    lock_held_in(plan::Preset::Paper, false, None, |m| no_sheet(software(m)))
}

#[test]
fn the_flow_after_the_lock_still_reads_the_held_description_as_the_text_file() {
    let (_shell, app) = restarted_held_flow();
    let wf = app.flow().unwrap().wallet.as_ref().unwrap();
    assert!(
        wf.shape.text_files,
        "the held shape is kept over the restart"
    );
    let (shape, _) = app.board_plan().unwrap();
    assert!(shape.text_files);
}

#[test]
fn the_board_of_the_flow_after_the_lock_marks_the_text_file_not_the_picture() {
    let (mut shell, mut app) = lock_held_in(plan::Preset::Paper, false, None, |m| {
        let m = plan::with_added(&software(m), At::Stick(0, 0), What::WalletFile(form::QR));
        no_sheet(m)
    });
    let i = app.backup_flows().len() - 1;
    shell.flow(&mut app, F::Show(i));
    shell.flow(&mut app, F::ChangePlan);
    if app.flow().unwrap().live.plan_q != Some(faraday_core::qstep::BOARD) {
        shell.flow(
            &mut app,
            F::Plan(faraday_core::PlanPress::Q(faraday_core::qstep::BOARD)),
        );
    }
    assert!(app.board_shown());
    let t = app.drawn_texts();
    assert!(t.iter().any(|x| x.contains("Text file")), "{t:?}");
    assert!(!t.iter().any(|x| x.contains("QR picture")), "{t:?}");
}

#[test]
fn the_krux_code_tool_refuses_a_held_silent_wallet_and_takes_it_once_the_backup_is_done() {
    let (mut app, _) = made_here();
    app.press(Action::SAddWallet);
    crate::common::open_kef_tool(&mut app);
    app.press(Action::KefTool(
        faraday_core::kef_tool::KefToolAction::Wallet(0),
    ));
    assert!(app.toast_text().is_some_and(|t| t.contains("No address")));
    assert!(
        faraday_core::kef_tool::state(&app)
            .unwrap()
            .form()
            .is_none()
    );
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    app.press(Action::KefTool(
        faraday_core::kef_tool::KefToolAction::Wallet(0),
    ));
    assert!(
        faraday_core::kef_tool::state(&app)
            .unwrap()
            .form()
            .is_some()
    );
}

#[test]
fn saving_a_held_silent_wallet_to_a_vault_stays_allowed_and_its_item_shows_the_address_once_released()
 {
    let (mut app, address) = made_here();
    app.press(Action::SAddWallet);
    crate::common::open_plain_vault(&mut app);
    app.press(Action::Vault(V::SaveWallet(0)));
    let shown = crate::common::vault_wallet_item_text(&mut app);
    assert!(shown.contains(WARNING), "{shown}");
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    let shown = crate::common::vault_wallet_item_text(&mut app);
    assert!(shown.contains(&address[..24]), "{shown}");
    assert!(!shown.contains("does not match"), "{shown}");
}
