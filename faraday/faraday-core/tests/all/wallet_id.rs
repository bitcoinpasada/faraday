//! A wallet is known by a hash of its whole descriptor, not by its
//! 8-character checksum: two different wallets that share a checksum are
//! two wallets for Check, for Spend from backup and for what is kept
//! across a lock (`docs/WALLETS.md` §4).
//!
//! The three descriptors below share one checksum (public test keys; found
//! once by solving the checksum's linear form over free digits of one
//! cosigner's origin). `TWIN` changes the threshold and that origin, so it
//! pays to other addresses; `ALIAS` changes only that origin, so it pays to
//! the same addresses as `FIRST`.

use faraday_core::spend_backup::{SpendBackupAction as F, page};
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::{Origin, step};
use faraday_core::{Action, Ask, Faraday, Screen, StorageCommand, StorageEvent, files_from};
use faraday_shell_api::App;

use faraday_core::testkit::{
    WALLET_ID_ALIAS as ALIAS, WALLET_ID_FIRST as FIRST, WALLET_ID_TWIN as TWIN,
};

fn kit(id: &'static str, descriptor: &str) -> Kit {
    Kit {
        id,
        name: id,
        descriptor: descriptor.to_string(),
        cosigner: None,
        fee: 2_000,
    }
}

fn first_and_twin() -> Vec<Kit> {
    vec![kit("first", FIRST), kit("twin", TWIN)]
}

fn first_and_alias() -> Vec<Kit> {
    vec![kit("first", FIRST), kit("alias", ALIAS)]
}

fn at(app: &Faraday, name: &str) -> usize {
    app.inbox().iter().position(|i| i.name == name).unwrap()
}

/// An app whose Files hold the wallet files and the spends of `kits`
/// (`<id>.psbt`), given what a process before kept.
fn with_files(kits: &[Kit], kept: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    let mut inbox: Vec<(String, Vec<u8>)> = kits.iter().map(testkit::wallet_file).collect();
    for k in kits {
        let psbt = testkit::unsigned(k).unwrap();
        inbox.push((format!("{}.psbt", k.id), psbt.to_bytes()));
    }
    app.storage(StorageEvent::Restored {
        inbox: files_from(inbox),
        outbox: Vec::new(),
        kept: faraday_core::kept_from(kept),
    });
    app
}

/// The wallets of `kits` loaded, in that order.
fn load(app: &mut Faraday, kits: &[Kit]) {
    for k in kits {
        let file = testkit::wallet_file(k).0;
        let wallet_at = at(app, &file);
        testkit::load_wallet(app, wallet_at);
    }
    assert_eq!(app.session.wallets.len(), kits.len());
}

/// What `app` keeps across a lock, as the next process is given it.
fn kept_of(app: &mut Faraday) -> Vec<(String, Vec<u8>)> {
    // A save of the Files and what is kept goes out with any change to
    // the idle lock.
    app.press(Action::IdleLock(5));
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k
                .into_iter()
                .map(|(n, b)| (n.to_string(), b.to_vec()))
                .collect();
        }
    }
    kept
}

/// Whether the spend of `name` opens with Check done.
fn check_done_on(app: &mut Faraday, name: &str) -> bool {
    let n = at(app, name);
    app.press(Action::StartSpend(n));
    app.spend().expect("a spend is open").done[step::CHECK as usize]
}

#[test]
fn the_wallets_of_one_checksum_load_together_as_two_wallets() {
    let kits = first_and_twin();
    let mut app = with_files(&kits, Vec::new());
    load(&mut app, &kits);
    let w = &app.session.wallets;
    assert_eq!(w[0].checksum(), w[1].checksum());
    assert_ne!(w[0].id(), w[1].id());
}

/// Check passed for one wallet is not Check passed for another that
/// shares its checksum.
#[test]
fn a_second_wallet_of_the_same_checksum_owes_check() {
    let kits = first_and_twin();
    let mut app = with_files(&kits, Vec::new());
    load(&mut app, &kits);
    assert!(!check_done_on(&mut app, "first.psbt"));
    app.press(Action::StepNext(step::CHECK));
    assert!(check_done_on(&mut app, "first.psbt"), "Check was passed");
    assert!(
        !check_done_on(&mut app, "twin.psbt"),
        "Check passed for another wallet of the same checksum"
    );
}

/// The spend opened again after its wallet was swapped for another of the
/// same checksum owes Check again: the first wallet is removed, and the
/// spend of the same coins, opened anew, is of the second. (This is the
/// reopened spend; the swap under an open spend is `keep_fresh`'s.)
#[test]
fn a_spends_wallet_change_owes_check_again() {
    let kits = first_and_alias();
    let mut app = with_files(&kits, Vec::new());
    load(&mut app, &kits);
    assert!(!check_done_on(&mut app, "first.psbt"));
    app.press(Action::StepNext(step::CHECK));
    assert!(check_done_on(&mut app, "first.psbt"));
    app.press(Action::RemoveWallet(0));
    assert_eq!(app.session.wallets.len(), 1);
    assert!(
        !check_done_on(&mut app, "first.psbt"),
        "Check carried to a wallet that was not shown"
    );
}

/// What a lock keeps is the wallets checked, and the wallet of the same
/// checksum is not among them.
#[test]
fn a_lock_keeps_the_two_wallets_apart() {
    let mut app = with_files(&first_and_twin(), Vec::new());
    load(&mut app, &first_and_twin());
    assert!(!check_done_on(&mut app, "first.psbt"));
    app.press(Action::StepNext(step::CHECK));
    let kept = kept_of(&mut app);
    assert!(kept.iter().any(|(n, _)| n == "checked-wallets"));

    let mut next = with_files(&first_and_twin(), kept);
    load(&mut next, &first_and_twin());
    assert!(check_done_on(&mut next, "first.psbt"), "kept for the first");
    assert!(
        !check_done_on(&mut next, "twin.psbt"),
        "kept for the second as well"
    );
}

/// A kept list from before, of 8-character checksums, names no wallet.
#[test]
fn a_kept_checksum_checks_no_wallet() {
    let kept = vec![("checked-wallets".to_string(), b"6kjg6089\n".to_vec())];
    let mut app = with_files(&[kit("first", FIRST)], kept);
    load(&mut app, &[kit("first", FIRST)]);
    assert!(!check_done_on(&mut app, "first.psbt"));
}

/// Spend from backup: the answer to Check the money was for the wallet
/// chosen; the wallet of the same checksum asks again.
#[test]
fn spend_from_backup_asks_the_money_check_again_for_the_twin() {
    let c = testkit::compute();
    let mut app = Faraday::new();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    for (name, d) in [("First", FIRST), ("Twin", TWIN)] {
        app.session
            .add_wallet_confirmed(&c, name, d, "kit", Origin::Named)
            .expect("the wallet reads");
    }
    assert_eq!(app.session.wallets.len(), 2);
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::Choose(0)));
    app.press(Action::SpendBackup(F::Next(page::CHECK)));
    assert!(app.spend_backup_page_done(page::CHECK));
    app.press(Action::SpendBackup(F::Choose(1)));
    assert_eq!(app.spend_backup_wallet(), Some(1));
    assert!(
        !app.spend_backup_page_done(page::CHECK),
        "the answer for one wallet carried to the other of its checksum"
    );
    app.press(Action::SpendBackup(F::Choose(0)));
    assert_eq!(app.spend_backup_wallet(), Some(0));
}

/// What the address check's page says.
fn said(app: &Faraday) -> String {
    app.address_check_state()
        .map(|p| {
            p.result_lines()
                .iter()
                .map(|l| String::from(l.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

/// A look for an address further than the first hundred ends when the
/// wallet in its place is swapped for another of the same checksum: the
/// place may hold a wallet the look did not begin in.
#[test]
fn an_address_look_ends_when_a_wallet_of_the_same_checksum_takes_the_place() {
    let first = [kit("first", FIRST)];
    let mut app = with_files(&first, Vec::new());
    load(&mut app, &first);
    let c = testkit::compute();
    let address = faraday_bip::policy::WalletPolicy::parse_any(&c, FIRST)
        .unwrap()
        .address_at(&c, testkit::net(), false, 150)
        .unwrap()
        .to_string();
    app.press(Action::ScanFor(Ask::Address));
    app.event(faraday_shell_api::Event::Scanned {
        bytes: address.into_bytes(),
    });
    let _ = app.frame();
    app.press(Action::AddressCheck(
        faraday_core::address_check::AddressPress::Way(0),
    ));
    app.event(faraday_shell_api::Event::Tick { now_ms: 1 });
    assert!(said(&app).contains("Looking"), "{}", said(&app));
    // The wallet goes and one of the same checksum takes its place.
    app.session
        .add_wallet_confirmed(&c, "twin", TWIN, "kit", Origin::Named)
        .expect("the wallet reads");
    app.session.wallets.remove(0);
    assert_eq!(app.session.wallets[0].checksum(), "6kjg6089");
    app.event(faraday_shell_api::Event::Tick { now_ms: 2 });
    // The twin's addresses are not the first's: a look that went on
    // would still say it is looking.
    assert!(
        !said(&app).contains("Looking"),
        "the look went on in a wallet it did not begin in: {}",
        said(&app)
    );
}

/// Check the money from a wallet's card: the answer for one wallet does
/// not carry to the card of the wallet of its checksum.
#[test]
fn spend_from_a_card_asks_the_money_check_again_for_the_twin() {
    let kits = first_and_twin();
    let mut app = with_files(&kits, Vec::new());
    load(&mut app, &kits);
    // The first wallet is checked, on a spend of it.
    app.press(Action::StartSpend(at(&app, "first.psbt")));
    app.press(Action::StepNext(step::CHECK));
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::SpendFrom(0)));
    assert!(
        app.spend_backup_page_done(page::CHECK),
        "the first is checked"
    );
    app.press(Action::SpendBackup(F::SpendFrom(1)));
    assert_eq!(app.spend_backup_wallet(), Some(1));
    assert!(
        !app.spend_backup_page_done(page::CHECK),
        "the answer for one wallet carried to the card of the other"
    );
}

/// A wallet file of the twin, taken while the first is chosen, starts the
/// flow again: Check and Write open and only that file came in.
#[test]
fn a_twins_file_taken_after_the_first_starts_the_flow_again() {
    let kits = first_and_twin();
    let mut app = with_files(&kits, Vec::new());
    app.press(Action::Nav(Screen::SpendBackup));
    let first = at(&app, &testkit::wallet_file(&kits[0]).0);
    app.press(Action::SpendBackup(F::FromFile(first)));
    testkit::confirm_wallets(&mut app);
    assert_eq!(app.spend_backup_wallet(), Some(0));
    app.press(Action::SpendBackup(F::Next(page::CHECK)));
    app.press(Action::SpendBackup(F::Next(page::WRITE)));
    assert!(app.spend_backup().closed[page::CHECK as usize]);
    assert!(app.spend_backup().closed[page::WRITE as usize]);
    let file = testkit::wallet_file(&kits[1]).0;
    let n = at(&app, &file);
    app.press(Action::SpendBackup(F::FromFile(n)));
    testkit::confirm_wallets(&mut app);
    assert_eq!(app.session.wallets.len(), 2, "the twin was not taken");
    assert_eq!(app.spend_backup_wallet(), Some(1));
    let s = app.spend_backup();
    assert!(!s.closed[page::CHECK as usize], "Check stayed done");
    assert!(!s.closed[page::WRITE as usize], "Write stayed done");
    assert!(
        matches!(&s.came[..], [faraday_core::spend_backup::Came::File(t)] if t.as_str() == file),
        "{:?}",
        s.came
    );
}

/// Backups lists two wallets of one checksum as two entries, and the
/// record of one is not shown for the other, with the first loaded
/// or after it is taken out.
#[test]
fn backups_keeps_the_record_of_one_twin_to_that_twin() {
    let kits = first_and_twin();
    let mut app = with_files(&kits, Vec::new());
    load(&mut app, &kits);
    let first = app.session.wallets[0].id().to_hex().to_string();
    let twin = app.session.wallets[1].id().to_hex().to_string();
    let sum = app.session.wallets[0].checksum().to_string();
    let record = format!("backup\t{sum}\tfirst\t1\nwid\t{first}\n");
    testkit::restore_backups(&mut app, record.as_bytes());
    let c = testkit::compute();
    let all = app.backups_now(&c);
    assert_eq!(all.len(), 2, "two wallets listed: {all:?}");
    let of = |id: &str| all.iter().find(|e| e.id == id).expect("an entry");
    assert!(of(&first).lines.is_some(), "the first has its record");
    assert!(of(&twin).lines.is_none(), "the twin has no record");
    // The first goes: the twin is listed alone, with no record.
    app.session.wallets.remove(0);
    let all = app.backups_now(&c);
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, *twin);
    assert!(all[0].lines.is_none(), "the twin took the first's record");
}
