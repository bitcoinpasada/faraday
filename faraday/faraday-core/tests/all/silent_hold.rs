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

/// `text` without its spaces: an address is drawn in groups of four.
fn packed(text: &str) -> String {
    text.replace(' ', "")
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
    assert!(!packed(&text).contains(&address[..24]), "{text}");
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
    assert!(!packed(&text).contains(&address[..24]), "{text}");
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
    // Neither the start nor the end of the address is shown.
    assert!(
        !packed(&text).contains(&address[address.len() - 4..]),
        "{text}"
    );
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    app.press(Action::SWallet(0));
    let text = drawn(&mut app);
    assert!(
        packed(&text).contains(&address[address.len() - 4..]),
        "{text}"
    );
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
    assert!(!packed(&shown).contains(&address[..24]), "{shown}");
}

#[test]
fn the_backup_done_shows_the_address_again_and_a_key_not_made_here_never_held_it() {
    let (mut app, address) = made_here();
    app.press(Action::SAddWallet);
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    app.press(Action::SWallet(0));
    let text = drawn(&mut app);
    assert!(packed(&text).contains(&address[..24]), "{text}");
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
    assert!(packed(&text).contains(&address[..24]), "{text}");
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
    edit: impl Fn(Vec<plan::Spot>) -> Vec<plan::Spot>,
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
    edit: impl Fn(Vec<plan::Spot>) -> Vec<plan::Spot>,
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
    edit: impl Fn(Vec<plan::Spot>) -> Vec<plan::Spot>,
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
    lock_held_from(shell, app, preset, text, only, edit)
}

/// Back up opened on `preset`, Sparrow, the files' form and the map as
/// `edit` leaves it.
fn set_up_plan(
    app: &mut Faraday,
    preset: Option<plan::Preset>,
    text: bool,
    only: Option<usize>,
    edit: &impl Fn(Vec<plan::Spot>) -> Vec<plan::Spot>,
) {
    app.press(Action::Backup(0));
    // A plan saved before is the one Back up opens on: no preset's presses
    // reach the plan from it.
    if let Some(preset) = preset {
        testkit::backup_as(app, preset);
    }
    app.press(Action::BAnswer(
        qrow::SOFTWARE,
        plan::software::SPARROW as u8,
    ));
    if text {
        app.press(Action::BAnswer(qrow::FORM, plan::form::TEXT as u8));
    }
    if let Some(k) = only {
        testkit::change_backup_answers(app, |a| {
            a.software = [false; 5];
            a.software[k] = true;
        });
    }
    let c = testkit::compute();
    let shape = app.plan_shape_now(app.backup().unwrap().wallet);
    testkit::change_backup_answers(app, |a| {
        let m = plan::map(&c, &shape, a);
        a.set_map(edit(m));
    });
}

/// [`lock_held_in`] from a Faraday that has the wallet already: Back up,
/// the plan edited, Lock; the flow after the restart.
fn lock_held_from(
    shell: Shell,
    app: Faraday,
    preset: plan::Preset,
    text: bool,
    only: Option<usize>,
    edit: impl Fn(Vec<plan::Spot>) -> Vec<plan::Spot>,
) -> (Shell, Faraday) {
    lock_held_with(shell, app, Some(preset), text, only, edit)
}

/// [`lock_held_from`] where `preset` is None: Back up opens on the plan
/// the wallet's last backup saved.
fn lock_held_with(
    shell: Shell,
    app: Faraday,
    preset: Option<plan::Preset>,
    text: bool,
    only: Option<usize>,
    edit: impl Fn(Vec<plan::Spot>) -> Vec<plan::Spot>,
) -> (Shell, Faraday) {
    lock_held_inner(shell, app, preset, text, only, edit, false)
}

/// [`lock_held_with`], the key released where `release`: once the files
/// are made, after the last time Backups' record was made (a paper copy
/// check of the checklist releases it there, before the lock).
fn lock_held_inner(
    mut shell: Shell,
    mut app: Faraday,
    preset: Option<plan::Preset>,
    text: bool,
    only: Option<usize>,
    edit: impl Fn(Vec<plan::Spot>) -> Vec<plan::Spot>,
    release: bool,
) -> (Shell, Faraday) {
    set_up_plan(&mut app, preset, text, only, &edit);
    let c = testkit::compute();
    app.press(Action::BChecklist);
    save_plan_vaults(&mut app);
    app.press(Action::BFilesMake);
    testkit::type_passphrases(&mut app);
    if release {
        let fp = app.session.keys[0].fingerprint;
        testkit::release_made(&mut app, fp);
        // Backups' record is still the one made while held.
        let h = backups_text(&app);
        assert!(h.contains("Text file"), "{h}");
        // The descriptor's code, now asked for, is shown from the
        // checklist.
        app.press(Action::QrWallet(0));
        app.press(Action::Cancel);
        let h = backups_text(&app);
        assert!(h.contains("Text file"), "{h}");
    } else if app
        .backup_items_now(&c)
        .contains(&plan::Item::ShowDescriptor)
    {
        // Not held: the descriptor's code is shown first.
        app.press(Action::QrWallet(0));
        // Back up opens on the plan saved last: the edits are made again.
        set_up_plan(&mut app, None, text, only, &edit);
        app.press(Action::BChecklist);
    }
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
    assert!(packed(&shown).contains(&address[..24]), "{shown}");
    assert!(!shown.contains("does not match"), "{shown}");
}

/// The held key with an ordinary wallet over it (wallet 0) and its silent
/// payments wallet (wallet 1), on the silent wallet's page.
fn held_with_an_ordinary_wallet() -> Faraday {
    let (mut app, _) = made_here();
    let fp = app.session.keys[0].fingerprint;
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    app.press(Action::Silent);
    app.press(Action::SAddWallet);
    app.press(Action::SWallet(1));
    app
}

/// Whether the page being shown offers `action`, after a frame is drawn.
fn offers(app: &mut Faraday, action: Action) -> bool {
    app.settle();
    let _ = app.frame();
    app.offers(action)
}

#[test]
fn a_held_silent_wallets_page_offers_the_keys_backup_and_back_returns_to_the_page() {
    let mut app = held_with_an_ordinary_wallet();
    assert!(offers(&mut app, Action::Backup(0)));
    assert!(drawn(&mut app).contains("Back up"));
    // The ordinary wallet's backup, which places the key's seed.
    app.press(Action::Backup(0));
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup().unwrap().wallet, 0);
    app.settle();
    let _ = app.frame();
    app.event(Event::Key(Key::Escape));
    assert_eq!(app.screen, Screen::Silent);
    assert!(offers(&mut app, Action::Backup(0)));
}

#[test]
fn finishing_the_keys_backup_releases_the_silent_wallets_address_and_takes_the_button_away() {
    let mut app = held_with_an_ordinary_wallet();
    assert!(drawn(&mut app).contains(WARNING));
    assert!(offers(&mut app, Action::Backup(0)));
    app.press(Action::Backup(0));
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    app.settle();
    let _ = app.frame();
    app.event(Event::Key(Key::Escape));
    assert_eq!(app.screen, Screen::Silent);
    assert!(!offers(&mut app, Action::Backup(0)));
    let text = drawn(&mut app);
    assert!(!text.contains(WARNING), "{text}");
    assert!(text.contains("Show as QR"), "{text}");
}

#[test]
fn an_address_not_held_has_no_backup_button() {
    let mut app = held_with_an_ordinary_wallet();
    let fp = app.session.keys[0].fingerprint;
    testkit::release_made(&mut app, fp);
    assert!(!offers(&mut app, Action::Backup(0)));
}

#[test]
fn a_held_silent_wallet_with_no_ordinary_wallet_of_its_key_has_no_backup_button() {
    // The silent wallet's own backup places no seed and releases nothing.
    let (mut app, _) = made_here();
    app.press(Action::SAddWallet);
    app.press(Action::SWallet(0));
    assert!(drawn(&mut app).contains(WARNING));
    assert!(!offers(&mut app, Action::Backup(0)));
}

#[test]
fn a_silent_record_with_no_loaded_key_has_no_backup_button() {
    let (mut app, _) = made_here();
    app.press(Action::SAddWallet);
    app.press(Action::Lock);
    app.press(Action::SWallet(0));
    assert!(!offers(&mut app, Action::Backup(0)));
}

/// The key typed again and its silent wallet added, not held.
fn reload_wallet(app: &mut Faraday) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    testkit::keep_published(app);
    app.press(Action::Silent);
    app.press(Action::SAddWallet);
}

/// A held wallet's backup with the picture form and the software only
/// (no sheet, no stick of a place with a picture), its sticks written and
/// its key released.
fn software_written_then_released() -> (Shell, Faraday) {
    let (mut shell, mut app) =
        lock_held_in(plan::Preset::Paper, false, None, |m| no_sheet(software(m)));
    write_all_sticks(&mut shell, &mut app);
    reload_wallet(&mut app);
    (shell, app)
}

#[test]
fn the_public_stick_written_as_the_text_file_stays_in_the_plan_saying_text_file_after_the_release()
{
    let (_shell, app) = software_written_then_released();
    assert!(!app.plan_shape_now(0).text_files, "released");
    let c = testkit::compute();
    let shape = app.plan_shape_said(0);
    let a = app.flow().unwrap().wallet.as_ref().unwrap().answers.clone();
    assert!(
        plan::public_stick(&c, &shape, &a),
        "the public stick left the plan"
    );
    assert_eq!(a.form_in(&shape), [false, true]);
}

/// The plan with the picture on place 0's stick and the software, no sheet.
fn picture_on_place(m: Vec<plan::Spot>) -> Vec<plan::Spot> {
    let m = plan::with_added(&software(m), At::Stick(0, 0), What::WalletFile(form::QR));
    no_sheet(m)
}

/// A held wallet's backup with the picture on a place's stick, its sticks
/// written and its key released: the places the text file went to.
fn written_then_released() -> (Shell, Faraday) {
    let (mut shell, mut app) = lock_held_in(plan::Preset::Paper, false, None, picture_on_place);
    write_all_sticks(&mut shell, &mut app);
    // The key is entered again after the restart, and nothing holds it:
    // the hold is released.
    reload_wallet(&mut app);
    (shell, app)
}

/// What the map of the plan keeps at place 0's stick, under the wallet's
/// shape as every reader gets it.
fn place_holds(app: &Faraday) -> Vec<What> {
    let c = testkit::compute();
    let shape = app.plan_shape_said(0);
    let a = app.flow().unwrap().wallet.as_ref().unwrap().answers.clone();
    plan::map(&c, &shape, &a)
        .into_iter()
        .filter(|s| s.at == At::Stick(0, 0))
        .flat_map(|s| s.holds.into_iter().map(|(w, _)| w))
        .collect()
}

#[test]
fn the_plan_says_text_file_where_one_was_written_after_the_hold_is_released() {
    let (_shell, app) = written_then_released();
    assert!(!app.plan_shape_now(0).text_files, "released");
    let h = place_holds(&app);
    assert!(h.contains(&What::WalletFile(form::TEXT)), "{h:?}");
    assert!(!h.contains(&What::WalletFile(form::QR)), "{h:?}");
}

#[test]
fn the_text_file_written_stays_over_a_restart() {
    let (mut shell, app) = written_then_released();
    let h = place_holds(&app);
    assert_eq!(h, vec![What::WalletFile(form::TEXT)], "{h:?}");
    let mut app = shell.restart();
    reload_wallet(&mut app);
    let h = place_holds(&app);
    assert_eq!(h, vec![What::WalletFile(form::TEXT)], "{h:?}");
}

#[test]
fn a_place_written_nothing_in_a_released_plan_reads_the_chosen_form() {
    // The sticks written carry no description (software only): nothing was
    // written as the text file, so the released plan reads the form the
    // plan chooses where the picture is added.
    let (_shell, mut app) = software_written_then_released();
    // The flow's own plan keeps no mark of a place.
    let h = place_holds(&app);
    assert!(!h.contains(&What::WalletFile(form::TEXT)), "{h:?}");
    let h = holds_after_a_drop(&mut app);
    assert!(
        h.iter().any(|(_, w)| *w == What::WalletFile(form::QR)),
        "{h:?}"
    );
    assert!(
        !h.iter().any(|(_, w)| *w == What::WalletFile(form::TEXT)),
        "{h:?}"
    );
}

#[test]
fn the_restarted_flow_edit_that_changes_nothing_keeps_the_public_stick() {
    let (mut shell, app) = restarted_held_flow();
    let mut app = app;
    let c = testkit::compute();
    let has_public = |app: &Faraday| {
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        plan::public_stick(&c, &wf.shape, &wf.answers)
    };
    let slots = app.flow().unwrap().slots.len();
    assert!(has_public(&app), "the public stick is in the plan");
    let before = app
        .flow()
        .unwrap()
        .wallet
        .as_ref()
        .unwrap()
        .answers
        .to_text();
    shell.flow(&mut app, F::Plan(faraday_core::PlanPress::Omit(0)));
    assert!(
        app.toast_text()
            .is_none_or(|t| !t.contains("stay as made after the lock")),
        "{:?}",
        app.toast_text()
    );
    assert!(has_public(&app), "the public stick is still in the plan");
    assert_eq!(app.flow().unwrap().slots.len(), slots, "the slots");
    let after = app
        .flow()
        .unwrap()
        .wallet
        .as_ref()
        .unwrap()
        .answers
        .to_text();
    assert_eq!(before, after);
}

#[test]
fn the_public_stick_reads_text_file_in_the_map_after_the_release() {
    let (_shell, app) = software_written_then_released();
    let c = testkit::compute();
    let a = app.flow().unwrap().wallet.as_ref().unwrap().answers.clone();
    assert!(!app.plan_shape_now(0).text_files, "released");
    let m = plan::Mapped::of(&c, &app.plan_shape_said(0), &a);
    assert!(m.text_files, "the public files read as the text file");
}

/// The files on the sticks of place 0 that name the descriptor, after the
/// sticks `shell` holds were written.
fn descriptor_files(shell: &Shell) -> Vec<String> {
    ["S1", "S2", "S3"]
        .iter()
        .flat_map(|d| shell.disk(d).files.iter().map(|f| f.0.clone()))
        .filter(|n| n.ends_with("-descriptor.txt") || n.ends_with("-descriptor.png"))
        .collect()
}

/// Done, the key typed again and not held, a new backup locked with the
/// picture on place 0 and its sticks written.
fn second_backup_written() -> (Shell, Faraday, Vec<String>) {
    let (mut shell, mut app) = written_then_released();
    let app_done = std::mem::replace(&mut app, shell.restart());
    drop(app_done);
    reload_wallet(&mut app);
    for d in ["S1", "S2", "S3"] {
        shell.disk_mut(d).files.clear();
    }
    let (mut shell, mut app) = lock_held_from(
        shell,
        app,
        plan::Preset::Paper,
        false,
        None,
        picture_on_place,
    );
    // The flow just locked is the last one kept.
    let last = app.backup_flows().len() - 1;
    shell.flow(&mut app, F::Show(last));
    write_all_sticks(&mut shell, &mut app);
    let files = descriptor_files(&shell);
    (shell, app, files)
}

#[test]
fn a_picture_chosen_after_a_text_file_was_written_is_made_as_the_picture() {
    let (mut shell, mut app, files) = second_backup_written();
    assert!(
        files.iter().any(|n| n.ends_with("-descriptor.png")),
        "{files:?}"
    );
    assert!(
        !files.iter().any(|n| n.ends_with("-descriptor.txt")),
        "{files:?}"
    );
    // Written as the picture: the map, the board and the shape every reader
    // gets say picture, none text.
    let h = place_holds(&app);
    assert!(h.contains(&What::WalletFile(form::QR)), "{h:?}");
    assert!(!h.contains(&What::WalletFile(form::TEXT)), "{h:?}");
    let t = board_texts_of_last_flow(&mut shell, &mut app);
    assert!(t.iter().any(|x| x.contains("QR picture")), "{t:?}");
    assert!(!t.iter().any(|x| x.contains("Text file")), "{t:?}");
}

/// The board of the flow past its lock, as the earlier board test reads it.
fn board_texts_of_last_flow(shell: &mut Shell, app: &mut Faraday) -> Vec<String> {
    let i = app.backup_flows().len() - 1;
    shell.flow(app, F::Show(i));
    shell.flow(app, F::ChangePlan);
    if app.flow().unwrap().live.plan_q != Some(faraday_core::qstep::BOARD) {
        shell.flow(
            app,
            F::Plan(faraday_core::PlanPress::Q(faraday_core::qstep::BOARD)),
        );
    }
    assert!(app.board_shown());
    app.drawn_texts().iter().map(|t| t.to_string()).collect()
}

/// What the backup's map keeps on the sticks of place 0 once a person has
/// dropped the description on a new stick there: each stick and the form of
/// its description file.
fn holds_after_a_drop(app: &mut Faraday) -> Vec<(At, What)> {
    use crate::common::put;
    use faraday_core::board::{Thing, Zone};
    app.press(Action::Backup(0));
    app.press(Action::BAnswer(qrow::FORM, plan::form::QR as u8));
    app.press(Action::BPlan);
    if app.backup().unwrap().q != Some(faraday_core::qstep::BOARD) {
        app.press(Action::BQ(faraday_core::qstep::BOARD));
    }
    // A drop from the list places the description as the text file (the
    // chip's first form); the person turns on the picture and off the text.
    let n = crate::common::backup_map(app)
        .iter()
        .filter(|s| matches!(s.at, At::Stick(0, _)))
        .count();
    put(app, Thing::Whole, Zone::NewStick(0));
    crate::common::form(app, Zone::Files(0, n), Thing::Whole, 1);
    crate::common::form(app, Zone::Files(0, n), Thing::Whole, 0);
    let c = testkit::compute();
    let shape = app.plan_shape_said(0);
    let a = app.backup().unwrap().answers.clone();
    plan::map(&c, &shape, &a)
        .into_iter()
        .filter(|s| matches!(s.at, At::Stick(0, _)))
        .flat_map(|s| {
            let at = s.at;
            s.holds.into_iter().map(move |(w, _)| (at, w))
        })
        .filter(|(_, w)| matches!(w, What::WalletFile(_)))
        .collect()
}

#[test]
fn a_place_whose_stick_held_only_a_vault_reads_the_picture_after_the_release() {
    // The plan has no descriptor file on place 0's stick: the vault's stick
    // is written, and no text file with it.
    let (mut shell, mut app) =
        lock_held_in(plan::Preset::Paper, false, None, |m| no_sheet(software(m)));
    write_all_sticks(&mut shell, &mut app);
    reload_wallet(&mut app);
    let h = holds_after_a_drop(&mut app);
    assert!(
        h.iter().any(|(_, w)| *w == What::WalletFile(form::QR)),
        "{h:?}"
    );
    assert!(
        !h.iter().any(|(_, w)| *w == What::WalletFile(form::TEXT)),
        "{h:?}"
    );
}

#[test]
fn a_stick_dropped_while_the_backup_is_made_and_not_done_adds_nothing() {
    // The flow still writes its sticks as the text file; the plan it follows
    // is locked, so a stick dropped now is not part of it and not the next
    // backup's either: the next backup starts when this one is done.
    let (_shell, mut app) =
        lock_held_in(plan::Preset::Paper, true, None, |m| no_sheet(software(m)));
    assert_ne!(app.flow().unwrap().step, Step::Done);
    reload_wallet(&mut app);
    let h = holds_after_a_drop(&mut app);
    assert!(h.is_empty(), "{h:?}");
}

#[test]
fn a_text_file_chosen_and_the_picture_added_later_reads_the_picture() {
    let (mut shell, mut app) =
        lock_held_in(plan::Preset::Paper, true, None, |m| no_sheet(software(m)));
    write_all_sticks(&mut shell, &mut app);
    reload_wallet(&mut app);
    let h = holds_after_a_drop(&mut app);
    assert!(
        h.iter().any(|(_, w)| *w == What::WalletFile(form::QR)),
        "{h:?}"
    );
    assert!(
        !h.iter().any(|(_, w)| *w == What::WalletFile(form::TEXT)),
        "{h:?}"
    );
}

/// The plan of place 0 as a vault with the wallet in it, the picture on
/// its stick and the software, no sheet.
fn picture_with_vault(m: Vec<plan::Spot>) -> Vec<plan::Spot> {
    picture_on_place(plan::with_added(&m, At::Vault(0), What::Wallet))
}

/// A lock refused because the plan no longer fits marks nothing as written:
/// the checklist made again saves the plan it saved before the lock.
#[test]
fn a_refused_lock_saves_no_mark_of_files_it_did_not_write() {
    let mut shell = Shell::light();
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    let (mut app, _) = made_here_in(app);
    app.press(Action::SAddWallet);
    set_up_plan(
        &mut app,
        Some(plan::Preset::PaperVault),
        false,
        None,
        &picture_with_vault,
    );
    app.press(Action::BChecklist);
    save_plan_vaults(&mut app);
    app.press(Action::BFilesMake);
    testkit::type_passphrases(&mut app);
    let checklist = |app: &mut Faraday| -> Vec<String> {
        app.press(Action::BPlan);
        app.press(Action::BChecklist);
        testkit::settle(app);
        crate::common::contents(app, 0)
            .of(faraday_vault::records::kind::PLAN)
            .map(|(_, r)| {
                let answers = r.text(faraday_vault::records::field::PLAN_ANSWERS);
                answers.unwrap_or_default().to_string()
            })
            .collect()
    };
    let before = checklist(&mut app);
    assert!(!before.is_empty());
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| s.lower_limit(0));
    let used = app.vaults().records(id).used(&testkit::compute());
    testkit::vault_edit(&mut app, id, |s| s.lower_limit(used));
    app.press(Action::BNext(plan::Item::Envelopes));
    shell.flow(&mut app, F::WalletLock);
    testkit::settle(&mut app);
    assert!(!app.restart_requested(), "the lock went through");
    assert!(
        app.toast_text()
            .is_some_and(|t| t.ends_with("More than a vault can hold")),
        "{:?}",
        app.toast_text()
    );
    testkit::vault_edit(&mut app, id, |s| s.lower_limit(used + 4_000));
    assert_eq!(checklist(&mut app), before);
}

/// The first vault's stick (the only one plugged), the vault opened from
/// the Vaults tab, the key typed again after it.
fn open_first_vault_then_key(shell: &mut Shell, app: &mut Faraday, vd: &str) {
    open_first_vault(shell, app, vd);
    reload_wallet(app);
}

/// [`open_first_vault_then_key`] without the key: the wallet is only in the
/// vault's records.
fn open_first_vault(shell: &mut Shell, app: &mut Faraday, vd: &str) {
    for d in ["S1", "S2", "S3"] {
        shell.pull(app, d);
    }
    shell.plug(app, vd);
    testkit::vault_speed(app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    // A vault unlocks with its stick pulled.
    shell.pull(app, vd);
    crate::common::type_text(app, "first vault phrase");
    app.press(Action::Vault(V::Unlock));
    for t in 1..80u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick {
            now_ms: 100_000 + t * 1000,
        });
    }
    if app.sheet == Some(faraday_core::Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadTicked));
    }
    testkit::keep_published(app);
    assert_eq!(app.vaults().open.len(), 1, "the first vault is open");
}

/// Back up's own map for the wallet, as drawn.
fn backup_map_texts(app: &mut Faraday) -> Vec<String> {
    app.press(Action::Backup(0));
    app.drawn_texts().iter().map(|t| t.to_string()).collect()
}

/// The stick of a plan with one vault that holds the vault file.
fn vault_stick(shell: &Shell) -> &'static str {
    ["S1", "S2", "S3"]
        .into_iter()
        .find(|d| shell.disk(d).files.iter().any(|f| f.0.ends_with(".kdbx")))
        .expect("a stick holds the vault")
}

/// A fresh process (nothing kept, every stick out) that has the vault
/// file of stick `vd` read off it.
fn power_on_with_vault(shell: &mut Shell, vd: &str) -> Faraday {
    let vault: Vec<(String, Vec<u8>)> = shell
        .disk(vd)
        .files
        .iter()
        .filter(|f| f.0.ends_with(".kdbx"))
        .cloned()
        .collect();
    let mut app = shell.power_off();
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vault.clone()),
        outbox: Vec::new(),
        kept: faraday_core::kept_from(Vec::<(String, Vec<u8>)>::new()),
    });
    crate::common::know_files(&mut app, &vault);
    app
}

/// A held backup with the picture and a vault, locked, its sticks written
/// and Done; then a fresh process with the vault opened and the key typed
/// (so not held).
fn written_held_then_power_off() -> (Shell, Faraday, &'static str) {
    let (mut shell, mut app) =
        lock_held_in(plan::Preset::PaperVault, false, None, picture_with_vault);
    write_all_sticks(&mut shell, &mut app);
    let vd = vault_stick(&shell);
    let mut app = power_on_with_vault(&mut shell, vd);
    open_first_vault_then_key(&mut shell, &mut app, vd);
    (shell, app, vd)
}

#[test]
fn a_text_file_written_while_held_is_the_places_form_after_a_power_off_and_the_vault_opened() {
    let (_shell, mut app, _) = written_held_then_power_off();
    let tb = backup_map_texts(&mut app);
    assert!(tb.iter().any(|x| x.contains("Text file")), "{tb:?}");
    assert!(!tb.iter().any(|x| x.contains("QR picture")), "{tb:?}");
}

#[test]
fn a_vault_taken_off_the_plan_after_a_lock_leaves_the_files_stick_reading_text_file() {
    let (mut shell, mut app) = lock_held_in(
        plan::Preset::PaperVault,
        false,
        None,
        vault_stick_and_picture_stick,
    );
    write_all_sticks(&mut shell, &mut app);
    // The form of the wallet's file the flow's own board has on the sticks.
    let files = |app: &Faraday| -> Vec<What> {
        let (shape, a) = app.board_plan().expect("the board");
        plan::map(&testkit::compute(), &shape, &a)
            .into_iter()
            .filter(|s| matches!(s.at, At::Stick(..)))
            .flat_map(|s| s.holds.into_iter().map(|(w, _)| w))
            .filter(|w| matches!(w, What::WalletFile(_)))
            .collect()
    };
    app.press(Action::BPlan);
    let before = files(&app);
    assert_eq!(
        before,
        vec![What::WalletFile(form::TEXT)],
        "written as text"
    );
    crate::common::vault_off(&mut app, 0);
    // The vault is off the plan; the files stick is still the text file.
    let after = files(&app);
    assert_eq!(after, vec![What::WalletFile(form::TEXT)], "{after:?}");
}

#[test]
fn a_picture_written_after_a_text_file_is_the_places_form_after_a_power_off_and_the_vault_opened() {
    let (shell, app, vd) = written_held_then_power_off();
    // The key is typed (not held): a new backup writes the picture.
    let mut shell = shell;
    let mut app = app;
    for d in ["S1", "S2", "S3"] {
        shell.pull(&mut app, d);
        shell.disk_mut(d).files.retain(|f| f.0.ends_with(".kdbx"));
    }
    let (mut shell, mut app) = lock_held_with(shell, app, None, false, None, picture_with_vault);
    write_all_sticks(&mut shell, &mut app);
    let mut app = power_on_with_vault(&mut shell, vd);
    open_first_vault_then_key(&mut shell, &mut app, vd);
    let tb = backup_map_texts(&mut app);
    assert!(tb.iter().any(|x| x.contains("QR picture")), "{tb:?}");
    assert!(!tb.iter().any(|x| x.contains("Text file")), "{tb:?}");
}

/// Place 0 as two sticks: the vault alone on the first, the descriptor's
/// picture and the software on the second; no sheet.
fn vault_stick_and_picture_stick(m: Vec<plan::Spot>) -> Vec<plan::Spot> {
    let m = plan::with_added(&m, At::Vault(0), What::Wallet);
    let m = plan::with_added(&software(m), At::Stick(0, 1), What::WalletFile(form::QR));
    no_sheet(m)
}

#[test]
fn a_vault_stick_written_without_the_text_file_reads_the_picture_after_a_power_off() {
    let (mut shell, mut app) = lock_held_in(
        plan::Preset::PaperVault,
        false,
        None,
        vault_stick_and_picture_stick,
    );
    write_all_sticks(&mut shell, &mut app);
    let vd = vault_stick(&shell);
    let names = |d: &str| -> Vec<String> {
        shell
            .disk(d)
            .files
            .iter()
            .map(|f| f.0.clone())
            .filter(|n| n.contains("-descriptor"))
            .collect()
    };
    assert!(names(vd).is_empty(), "the vault's stick: {:?}", names(vd));
    let other: Vec<String> = ["S1", "S2", "S3"]
        .into_iter()
        .filter(|d| *d != vd)
        .flat_map(names)
        .collect();
    assert!(
        other.iter().any(|n| n.ends_with("-descriptor.txt")),
        "{other:?}"
    );
    let mut app = power_on_with_vault(&mut shell, vd);
    open_first_vault_then_key(&mut shell, &mut app, vd);
    // A file dropped on a new stick reads the picture; the stick it was
    // written on keeps the text file.
    let h = holds_after_a_drop(&mut app);
    assert!(
        h.contains(&(At::Stick(0, 1), What::WalletFile(form::TEXT))),
        "{h:?}"
    );
    assert!(
        h.iter().any(|(_, w)| *w == What::WalletFile(form::QR)),
        "{h:?}"
    );
}

/// What the map of the backup being planned keeps at `at`, under the
/// shape every reader gets.
fn backup_holds(app: &mut Faraday, at: At) -> Vec<What> {
    let c = testkit::compute();
    let shape = app.plan_shape_said(0);
    let a = app.backup().unwrap().answers.clone();
    plan::map(&c, &shape, &a)
        .into_iter()
        .filter(|s| s.at == at)
        .flat_map(|s| s.holds.into_iter().map(|(w, _)| w))
        .collect()
}

#[test]
fn a_stick_removed_from_a_place_with_two_sticks_takes_its_text_file_mark_with_it() {
    use crate::common::{board, put};
    use faraday_core::board::{BoardPress, Thing, Zone};
    let (mut shell, mut app) = lock_held_in(plan::Preset::Paper, false, None, picture_on_place);
    write_all_sticks(&mut shell, &mut app);
    reload_wallet(&mut app);
    // A new backup: the description on two sticks of place 0 as a person
    // drops it, the second with the picture instead of the text file.
    app.press(Action::Backup(0));
    app.press(Action::BAnswer(qrow::FORM, plan::form::QR as u8));
    app.press(Action::BPlan);
    if app.backup().unwrap().q != Some(faraday_core::qstep::BOARD) {
        app.press(Action::BQ(faraday_core::qstep::BOARD));
    }
    put(&mut app, Thing::Whole, Zone::NewStick(0));
    put(&mut app, Thing::Whole, Zone::NewStick(0));
    board(
        &mut app,
        BoardPress::Form(Zone::Files(0, 1), Thing::Whole, 1),
    );
    board(
        &mut app,
        BoardPress::Form(Zone::Files(0, 1), Thing::Whole, 0),
    );
    let first = backup_holds(&mut app, At::Stick(0, 0));
    assert_eq!(first, vec![What::WalletFile(form::TEXT)], "written there");
    let second = backup_holds(&mut app, At::Stick(0, 1));
    assert_eq!(second, vec![What::WalletFile(form::QR)], "chosen");
    // The written stick goes; the second is the place's first now and was
    // written nothing.
    board(&mut app, BoardPress::RemoveStick(0, 0));
    let h = backup_holds(&mut app, At::Stick(0, 0));
    assert_eq!(h, vec![What::WalletFile(form::QR)], "{h:?}");
}

/// What Backups' record keeps of the wallet's places, as text.
fn backups_text(app: &Faraday) -> String {
    String::from_utf8_lossy(&testkit::backups_record(app)).to_string()
}

/// A held silent wallet's backup with the picture on a stick, the checklist
/// and the files made while the key is held and the key released before the
/// lock.
fn locked_after_release() -> (Shell, Faraday) {
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
    lock_held_inner(
        shell,
        app,
        Some(plan::Preset::Paper),
        false,
        None,
        |m| {
            no_sheet(plan::with_added(
                &m,
                At::Stick(0, 0),
                What::WalletFile(form::QR),
            ))
        },
        true,
    )
}

#[test]
fn backups_says_the_picture_a_lock_wrote_after_the_key_was_released_before_it() {
    let (mut shell, app) = locked_after_release();
    let h = backups_text(&app);
    assert!(h.contains("QR picture"), "{h}");
    assert!(!h.contains("Text file"), "{h}");
    let app = shell.restart();
    let h = backups_text(&app);
    assert!(h.contains("QR picture"), "{h}");
    assert!(!h.contains("Text file"), "{h}");
}

/// A new backup opened on the plan a vault keeps, the plan of a lock made
/// while held: the software and a stick marked as the text file.
fn new_backup_on_the_written_plan() -> Faraday {
    let (_shell, mut app, _) = written_held_then_power_off();
    app.press(Action::Backup(0));
    app
}

/// A plan with a stick written as the text file while held, and a second
/// place's stick given the picture afterwards; the board opened on it with
/// the key not held. The forms read as text only where the plan is not
/// custom, so that is set here; with its toggles on, a chip shows the
/// chosen form instead.
fn board_of_the_written_plan() -> Faraday {
    let mut app = new_backup_on_the_written_plan();
    testkit::change_backup_answers(&mut app, |a| {
        let m = a.map.clone().expect("a stored map");
        a.set_map(plan::with_added(
            &m,
            At::Stick(1, 0),
            What::WalletFile(form::QR),
        ));
        a.custom = false;
    });
    app.press(Action::BPlan);
    if app.backup().unwrap().q != Some(faraday_core::qstep::BOARD) {
        app.press(Action::BQ(faraday_core::qstep::BOARD));
    }
    assert!(app.board_shown());
    let (shape, a) = app.board_plan().unwrap();
    assert!(!shape.text_files, "the key is not held");
    assert!(!a.custom, "the chips are read only");
    assert!(a.written_on(At::Stick(0, 0)), "{:?}", a.map);
    assert!(!a.written_on(At::Stick(1, 0)), "{:?}", a.map);
    app
}

#[test]
fn the_board_says_text_file_on_the_written_stick_and_the_picture_on_the_other() {
    let mut app = board_of_the_written_plan();
    let t: Vec<String> = app.drawn_texts().iter().map(|t| t.to_string()).collect();
    let count = |w: &str| t.iter().filter(|x| x.as_str() == w).count();
    assert_eq!(count("· Text file"), 1, "{t:?}");
    assert_eq!(count("· QR picture"), 1, "{t:?}");
}

/// With its toggles on, the chip of the stick written as the text file
/// keeps the picture as the chosen form and says what was written, and so
/// do the chips of the software and of its stick of files, which have no
/// form; the other stick's picture and an ordinary wallet's say nothing
/// of it.
#[test]
fn the_toggled_chip_keeps_the_picture_ticked_and_says_written_as_text_file() {
    const LINE: &str = "Written as: Text file";
    let (mut shell, mut app) = lock_held_in(plan::Preset::PaperVault, false, None, |m| {
        plan::with_added(&picture_with_vault(m), At::Files, What::Wallet)
    });
    write_all_sticks(&mut shell, &mut app);
    let vd = vault_stick(&shell);
    let mut app = power_on_with_vault(&mut shell, vd);
    open_first_vault_then_key(&mut shell, &mut app, vd);
    app.press(Action::Backup(0));
    testkit::change_backup_answers(&mut app, |a| {
        let m = a.map.clone().expect("a stored map");
        a.set_map(plan::with_added(
            &m,
            At::Stick(1, 0),
            What::WalletFile(form::QR),
        ));
    });
    app.press(Action::BPlan);
    if app.backup().unwrap().q != Some(faraday_core::qstep::BOARD) {
        app.press(Action::BQ(faraday_core::qstep::BOARD));
    }
    assert!(app.board_shown());
    let (_, a) = app.board_plan().unwrap();
    assert!(a.custom, "the chips have their toggles");
    assert!(a.written_on(At::Software), "{:?}", a.map);
    assert!(a.written_on(At::Files), "{:?}", a.map);
    assert!(a.written_on(At::Stick(0, 0)), "{:?}", a.map);
    assert!(!a.written_on(At::Stick(1, 0)), "{:?}", a.map);
    // The stored map, which the board's toggles read: the chosen form.
    let stored = a.map.clone().expect("a stored map");
    let h: Vec<What> = stored
        .iter()
        .filter(|s| s.at == At::Stick(0, 0))
        .flat_map(|s| s.holds.iter().map(|h| h.0))
        .collect();
    assert!(h.contains(&What::WalletFile(form::QR)), "{h:?}");
    assert!(!h.contains(&What::WalletFile(form::TEXT)), "{h:?}");
    let t: Vec<String> = app.drawn_texts().iter().map(|t| t.to_string()).collect();
    // The marked stick's chip; the other stick's has the toggles and no
    // line.
    assert_eq!(t.iter().filter(|x| *x == LINE).count(), 1, "{t:?}");
    assert!(t.iter().any(|x| x == "QR picture"), "{t:?}");
    // The software's card is at the foot of the page: its chip and its
    // files stick's each have the line.
    app.event(Event::Scroll {
        x: 600,
        y: 400,
        dy: 20_000,
    });
    app.settle();
    let t: Vec<String> = app.drawn_texts().iter().map(|t| t.to_string()).collect();
    let soft = t.iter().position(|x| x == "Sparrow").expect("its card");
    assert_eq!(t[soft..].iter().filter(|x| *x == LINE).count(), 2, "{t:?}");

    // Wallet 0 is the ordinary one.
    let mut app = held_with_an_ordinary_wallet();
    let h = holds_after_a_drop(&mut app);
    assert!(
        h.iter().any(|(_, w)| *w == What::WalletFile(form::QR)),
        "{h:?}"
    );
    assert!(app.board_plan().unwrap().1.custom);
    let t: Vec<String> = app.drawn_texts().iter().map(|t| t.to_string()).collect();
    assert!(t.iter().any(|x| x == "QR picture"), "{t:?}");
    assert!(!t.iter().any(|x| x.starts_with("Written as")), "{t:?}");
    // Nor does the software's chip, unmarked.
    testkit::change_backup_answers(&mut app, |a| {
        let m = a.map.clone().expect("a stored map");
        a.set_map(plan::with_added(&m, At::Software, What::Wallet));
    });
    app.event(Event::Scroll {
        x: 600,
        y: 400,
        dy: 20_000,
    });
    app.settle();
    let t: Vec<String> = app.drawn_texts().iter().map(|t| t.to_string()).collect();
    let soft = t.iter().position(|x| x == "Scanned in").expect("its card");
    assert!(t[soft..].iter().any(|x| x == "Whole"), "{t:?}");
    assert!(!t.iter().any(|x| x.starts_with("Written as")), "{t:?}");
}

/// A held silent wallet's picture written with the watch-only software
/// only, Done, the key typed again (not held); then Back up again with the
/// text file ticked too, and the public files item of its checklist open.
fn text_ticked_after_a_marked_picture() -> Faraday {
    let (mut shell, mut app) = lock_held_in(
        plan::Preset::PaperVault,
        false,
        Some(plan::software::NOT_SURE),
        |m| no_sheet(software(plan::with_added(&m, At::Vault(0), What::Wallet))),
    );
    write_all_sticks(&mut shell, &mut app);
    let vd = vault_stick(&shell);
    let mut app = power_on_with_vault(&mut shell, vd);
    open_first_vault_then_key(&mut shell, &mut app, vd);
    app.press(Action::Backup(0));
    app.press(Action::BAnswer(qrow::FORM, plan::form::TEXT as u8));
    let a = app.backup().unwrap().answers.clone();
    assert!(a.written_on(At::Software), "{:?}", a.map);
    assert_eq!(a.form, [true, true]);
    app.press(Action::BChecklist);
    save_plan_vaults(&mut app);
    let c = testkit::compute();
    let items = app.backup_items_now(&c);
    assert!(items.contains(&plan::Item::PublicFiles), "{items:?}");
    if app.backup().unwrap().open != Some(plan::Item::PublicFiles) {
        app.press(Action::BStep(plan::Item::PublicFiles));
    }
    app
}

/// The descriptor's row keeps its picture's buttons: what is made takes
/// both forms chosen, though what was written was the text file.
#[test]
fn the_public_files_item_offers_the_descriptors_picture_where_the_written_one_was_text() {
    let mut app = text_ticked_after_a_marked_picture();
    let _ = app.frame();
    let offered = app.drawn_actions();
    assert!(offered.contains(&Action::PublicOut(0, 8)), "{offered:?}");
}

#[test]
fn backups_names_the_text_file_where_the_old_backup_wrote_it_and_not_the_picture() {
    let mut app = new_backup_on_the_written_plan();
    app.press(Action::BChecklist);
    let rec = backups_text(&app);
    assert!(rec.contains("-descriptor.txt"), "{rec}");
    assert!(!rec.contains("-descriptor.png"), "{rec}");
}

#[test]
fn a_silent_plan_without_a_mark_is_mapped_once_for_both_shapes() {
    let (app, _) = made_here();
    let mut app = app;
    app.press(Action::SAddWallet);
    app.press(Action::Backup(0));
    let a = app.backup().unwrap().answers.clone();
    assert!(!a.written_text());
    assert_eq!(
        testkit::plan_maps_for_the_make_shape_after_the_said(&app, 0, &a),
        0
    );
}
