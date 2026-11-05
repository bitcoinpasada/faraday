//! A save takes a codex32 share the plan keeps in another vault out only
//! where the string is held elsewhere (`docs/CODEX32.md` §9.5): by this
//! session or by another open vault. Against BIP 93's vector 2, whose key
//! is typed in from strings A and C: the session holds those, and not D.

use crate::codex32_kept::{
    A, C, D, FP, PHRASE, ix, key_from_strings, share_record, share_records, unlock_on_vaults,
    vault_of, wallet_and_backup,
};
use crate::common::*;
use faraday_core::board::Thing;
use faraday_core::plan;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_vault::codex32::{SetFacts, State};

const OTHER: &str = "other phrase";

/// Two open vaults, the first holding D, the second nothing; the session
/// knows the set and holds no string; Back up's plan keeps D in its first
/// vault and the seed in its second.
fn two_vaults_holding_d() -> Faraday {
    let mut app = device(vec![
        (
            "home.kdbx".into(),
            vault_of(PHRASE, vec![share_record(D)], 7),
        ),
        ("other.kdbx".into(), vault_of(OTHER, Vec::new(), 9)),
    ]);
    key_without_strings(&mut app);
    unlock_on_vaults(&mut app, "home.kdbx", PHRASE);
    unlock_on_vaults(&mut app, "other.kdbx", OTHER);
    wallet_and_backup(&mut app);
    assert!(!session_holds_d(&app), "the session holds D");
    vault_at(&mut app, 0, 0, &[]);
    vault_at(&mut app, 0, 1, &[]);
    into_vault(&mut app, 0, Thing::String(0, ix('d')));
    into_vault(&mut app, 1, Thing::String(0, ix('a')));
    app
}

/// The key as a seed of words with its 2-of-3 set known and no string of
/// it held: typed in from A and C, then the strings forgotten.
fn key_without_strings(app: &mut Faraday) {
    key_from_strings(app);
    let k = app.session.keys.position_of(FP).unwrap();
    let facts = |id: &[u8; 4]| SetFacts {
        threshold: 2,
        identifier: *id,
        length: 48,
        unknown: 0,
        strings: vec![
            (b'a', State::Made),
            (b'c', State::Made),
            (b'd', State::Made),
        ],
    };
    assert!(app.session.keys.set_codex32(k, Some(facts(b"zzzz"))));
    assert!(app.session.keys.set_codex32(k, Some(facts(b"name"))));
}

fn session_holds_d(app: &Faraday) -> bool {
    let k = app.session.keys.position_of(FP).unwrap();
    app.session.keys.holds_codex32(k, b'd')
}

fn holds(app: &Faraday, o: usize, s: &str) -> bool {
    share_records(app, o).iter().any(|(x, _)| x == s.as_bytes())
}

/// D moved from the first vault into the second, which the plan gives the seed:
/// the save of the second does not take D out, as nothing else holds it.
#[test]
fn a_share_moved_to_a_vault_is_not_taken_out_when_nothing_else_holds_it() {
    let mut app = two_vaults_holding_d();
    testkit::vault_current(&mut app, 0);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(0)));
    app.press(Action::Vault(V::Item(0)));
    app.press(Action::Vault(V::MoveTo));
    app.press(Action::Vault(V::CopyPick(0)));
    app.press(Action::Vault(V::CopyGo));
    assert!(holds(&app, 1, D), "D did not arrive");
    testkit::vault_current(&mut app, 1);
    app.press(Action::Nav(Screen::Backup));
    app.press(Action::BVaultSave(1));
    assert!(holds(&app, 1, D), "the save took D out of every vault");
    let c = testkit::compute();
    assert!(
        !app.backup_item_done_now(&c, plan::Item::Vault(1)),
        "the item reads done with D left in it"
    );
}

/// The chart puts D in a second vault that is closed, D being in the first
/// vault and not held by the session: the save of the first vault keeps D.
#[test]
fn a_share_the_chart_puts_in_a_closed_vault_stays_when_the_session_lacks_it() {
    let mut app = device(vec![(
        "home.kdbx".into(),
        vault_of(PHRASE, vec![share_record(D)], 7),
    )]);
    key_without_strings(&mut app);
    unlock_on_vaults(&mut app, "home.kdbx", PHRASE);
    wallet_and_backup(&mut app);
    assert!(!session_holds_d(&app), "the session holds D");
    vault_at(&mut app, 0, 0, &[]);
    vault_at(&mut app, 0, 1, &[]);
    into_vault(&mut app, 0, Thing::String(0, ix('a')));
    into_vault(&mut app, 1, Thing::String(0, ix('d')));
    app.press(Action::BVaultSave(0));
    assert!(holds(&app, 0, D), "the save took D out of the only vault");
}

/// The session holds the string: the share goes from the vault the plan
/// does not keep it in.
#[test]
fn a_share_the_session_holds_goes_from_the_vault_the_plan_does_not_keep_it_in() {
    let mut app = device(vec![(
        "home.kdbx".into(),
        vault_of(PHRASE, vec![share_record(C)], 7),
    )]);
    unlock_on_vaults(&mut app, "home.kdbx", PHRASE);
    key_from_strings(&mut app);
    wallet_and_backup(&mut app);
    vault_at(&mut app, 0, 0, &[]);
    vault_at(&mut app, 0, 1, &[]);
    into_vault(&mut app, 0, Thing::String(0, ix('a')));
    into_vault(&mut app, 1, Thing::String(0, ix('c')));
    app.press(Action::BVaultSave(0));
    assert!(!holds(&app, 0, C), "C stayed");
    assert!(holds(&app, 0, A), "A was not saved");
}

/// Two open vaults, the first holding D, the second nothing; the session
/// holds A and C and not D; Back up's plan keeps A in its first vault and
/// D in its second.
fn d_left_in_the_first() -> Faraday {
    let mut app = device(vec![
        (
            "home.kdbx".into(),
            vault_of(PHRASE, vec![share_record(D)], 7),
        ),
        ("other.kdbx".into(), vault_of(OTHER, Vec::new(), 9)),
    ]);
    key_without_strings(&mut app);
    unlock_on_vaults(&mut app, "home.kdbx", PHRASE);
    unlock_on_vaults(&mut app, "other.kdbx", OTHER);
    let k = app.session.keys.position_of(FP).unwrap();
    app.session
        .keys
        .keep_codex32(&testkit::compute(), k, &[A, C]);
    wallet_and_backup(&mut app);
    assert!(
        app.session.keys.holds_codex32(k, b'a'),
        "the session lacks A"
    );
    assert!(!session_holds_d(&app), "the session holds D");
    vault_at(&mut app, 0, 0, &[]);
    vault_at(&mut app, 0, 1, &[]);
    into_vault(&mut app, 0, Thing::String(0, ix('a')));
    into_vault(&mut app, 1, Thing::String(0, ix('d')));
    app
}

/// The first item of the open vault `from`, moved into the other of two.
fn move_first(app: &mut Faraday, from: usize) {
    testkit::vault_current(app, from);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(0)));
    app.press(Action::Vault(V::Item(0)));
    app.press(Action::Vault(V::MoveTo));
    app.press(Action::Vault(V::CopyPick(0)));
    app.press(Action::Vault(V::CopyGo));
    app.press(Action::Nav(Screen::Backup));
}

/// D stays in the first vault, which the plan keeps it out of, as nothing
/// else holds it: that vault reads done, and the vault the plan keeps D in
/// names what it lacks and stays undone until D is there.
#[test]
fn a_share_left_in_its_vault_does_not_hold_up_that_vaults_item() {
    let mut app = d_left_in_the_first();
    testkit::vault_current(&mut app, 0);
    app.press(Action::BVaultSave(0));
    assert_eq!(
        app.toast_text(),
        Some("Share D of name stays in this vault: nothing else holds it")
    );
    assert!(holds(&app, 0, D), "D was taken out");
    assert!(holds(&app, 0, A), "A was not saved");
    let c = testkit::compute();
    assert!(app.backup_item_done_now(&c, plan::Item::Vault(0)));
    assert!(!app.backup_item_done_now(&c, plan::Item::Vault(1)));
    assert_ne!(
        app.backup_waiting(&c),
        Some(faraday_core::bstep::of(plan::Item::Vault(0)))
    );
    testkit::vault_current(&mut app, 1);
    app.press(Action::BVaultSave(1));
    assert_eq!(
        app.toast_text(),
        Some("Share D of name is not here: type it in its Codex32 strings item first")
    );
    assert!(!app.backup_item_done_now(&c, plan::Item::Vault(1)));
    // D moved by hand into the second vault: that vault reads done.
    testkit::vault_current(&mut app, 0);
    move_first(&mut app, 0);
    assert!(holds(&app, 1, D), "D did not arrive");
    assert!(app.backup_item_done_now(&c, plan::Item::Vault(1)));
}

/// A share the plan keeps in the first vault, moved by hand into the second,
/// which the plan gives the seed: the second vault keeps it and reads done;
/// the first names the string it lacks and stays undone.
#[test]
fn a_share_moved_out_of_its_vault_leaves_that_vault_undone_and_the_other_done() {
    let mut app = device(vec![
        (
            "home.kdbx".into(),
            vault_of(PHRASE, vec![share_record(A)], 7),
        ),
        ("other.kdbx".into(), vault_of(OTHER, Vec::new(), 9)),
    ]);
    key_without_strings(&mut app);
    unlock_on_vaults(&mut app, "home.kdbx", PHRASE);
    unlock_on_vaults(&mut app, "other.kdbx", OTHER);
    wallet_and_backup(&mut app);
    vault_at(&mut app, 0, 0, &[]);
    vault_at(&mut app, 0, 1, &[0]);
    into_vault(&mut app, 0, Thing::String(0, ix('a')));
    move_first(&mut app, 0);
    assert!(holds(&app, 1, A), "A did not arrive");
    testkit::vault_current(&mut app, 1);
    app.press(Action::BVaultSave(1));
    if app.sheet == Some(faraday_core::Sheet::SaveAlso) {
        app.press(Action::Vault(V::SaveAlsoContinue));
    }
    if app.sheet == Some(faraday_core::Sheet::SaveKeeps) {
        app.press(Action::Vault(V::KeepSave));
    }
    assert!(holds(&app, 1, A), "the save took A out of every vault");
    let c = testkit::compute();
    assert!(app.backup_item_done_now(&c, plan::Item::Vault(1)));
    testkit::vault_current(&mut app, 0);
    app.press(Action::BVaultSave(0));
    assert_eq!(
        app.toast_text(),
        Some("Share A of name is not here: type it in its Codex32 strings item first")
    );
    assert!(!app.backup_item_done_now(&c, plan::Item::Vault(0)));
}
