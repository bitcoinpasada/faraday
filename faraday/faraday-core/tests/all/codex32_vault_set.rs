//! A vault holds exactly the plan's codex32 strings of the plan's set
//! (`docs/CODEX32.md` §9.5), as a person backing up would notice it,
//! against BIP 93's vector 2 (`tools/vectors/bip93/vectors.json`): a
//! string put into a vault after the lock sends that vault back to be
//! filled and written, and its sealed copy holds it; a string moved out of
//! a vault after the lock leaves it; after a new set is made for the seed,
//! the vault holds the new set's string and not the old set's, and that
//! string with the new set's paper strings gives the seed back.

use crate::backup_flow_wallet::{
    PHRASE as FLOW_PHRASE, bring_in_all, create_with_vaults, flow_board, flow_into_vault,
    flow_vault_zone, ofv, restart, save_vaults, step, sticks, write_sticks,
};
use crate::codex32_kept::{
    A, C, D, FP, PHRASE, codex32_item, ix, key_from_strings, share_records, type_each_back,
    unlock_on_vaults, vault_of, wallet_and_backup,
};
use crate::common::*;
use faraday_bip::codex32::Codex32;
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::board::{BoardPress, Thing};
use faraday_core::codex32_backup::{Press, SetPress, SetStep};
use faraday_core::plan::{self, At, What};
use faraday_core::{Action, Faraday, bstep, cstep, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_ui::keyboard::OskPress;
use faraday_vault::records::{field, kind};

const BIP93: &str = include_str!("../../../../tools/vectors/bip93/vectors.json");
/// Vector 2's secret, the seed the set splits.
const SECRET: &str = "MS12NAMES6XQGUZTTXKEQNJSJZV4JV3NZ5K3KWGSPHUH6EVW";

/// The codex32 strings `bytes`, a vault file, holds once opened with
/// `phrase`.
fn sealed_strings(bytes: &[u8], phrase: &str) -> Vec<Vec<u8>> {
    let c = testkit::compute();
    let (_, contents) = faraday_vault::open(&c, bytes, phrase.as_bytes()).unwrap();
    contents
        .of(kind::CODEX32_SHARE)
        .map(|(_, r)| r.field(field::CODEX32_STRING).unwrap_or_default().to_vec())
        .collect()
}

/// A single-key wallet over vector 2's key, typed in from strings A and C
/// inside Make a new wallet with one vault on stick S1, the plan as it
/// opens (the seed and the description in that vault, a string on paper
/// at each place): the checklist done with `in_vault`'s
/// strings in the vault, locked, and stopped on the writes.
fn locked_with(shell: &mut Shell, in_vault: &[char]) -> Faraday {
    sticks(shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(shell, &mut app, &["S1"]);
    create_vault(shell, &mut app, 0, FLOW_PHRASE);
    key_from_strings(&mut app);
    app.press(Action::Nav(faraday_core::Screen::Create));
    app.press(Action::CSlotHere(0, FP));
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    no_paper_description(&mut app);
    vault_at(&mut app, 0, 0, &[0]);
    for &j in in_vault {
        into_vault(&mut app, 0, Thing::String(0, ix(j)));
    }
    codex32_item(&mut app);
    type_each_back(&mut app);
    save_vaults(&mut app);
    let mut strings = share_records(&app, 0)
        .into_iter()
        .map(|(s, _)| s)
        .collect::<Vec<_>>();
    strings.sort();
    let mut want: Vec<Vec<u8>> = in_vault
        .iter()
        .map(|&j| of_index(j).as_bytes().to_vec())
        .collect();
    want.sort();
    assert_eq!(strings, want, "the vault holds the plan's strings");
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let app = restart(shell, &mut app);
    assert!(app.flow().unwrap().made);
    app
}

/// Vector 2's string at index `j`.
fn of_index(j: char) -> &'static str {
    match j {
        'a' => A,
        'c' => C,
        _ => D,
    }
}

/// The round after the lock run to its end: the vault unlocked; this
/// power-on holds no string of the set, so the key is typed in from
/// strings A and C and `typed` from their paper too; the vault filled,
/// locked again and S1 written. The copy of the vault S1 holds.
fn round_and_write(shell: &mut Shell, app: &mut Faraday, typed: &[&str]) -> Vec<u8> {
    assert!(
        app.flow().unwrap().adding,
        "a change of the vault is a round"
    );
    assert_eq!(step(app), Step::Vaults);
    unlock(app, FLOW_PHRASE);
    assert_eq!(step(app), Step::Make, "{:?}", unlock_error(app));
    key_from_strings(app);
    app.press(Action::Nav(faraday_core::Screen::BackupFlow));
    let k = app.session.keys.position_of(FP).unwrap();
    assert_eq!(app.session.keys.keep_codex32(k, typed), typed.len());
    shell.flow(app, F::CopyKey);
    shell.flow(app, F::MadeContinue);
    assert_eq!(step(app), Step::Lock, "{:?}", app.toast_text());
    shell.flow(app, F::LockWrite);
    let mut app = restart(shell, app);
    write_sticks(shell, &mut app, &["S1"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    ofv(shell, "S1").expect("Vault 1 on S1")
}

/// A codex32 string put into a vault after the lock sends that vault back
/// to be filled and written: its sealed copy holds the string.
#[test]
fn a_codex32_string_put_into_a_vault_after_the_lock_is_sealed_in_it() {
    let mut shell = Shell::new();
    let mut app = locked_with(&mut shell, &[]);
    assert_eq!(step(&app), Step::Write);
    shell.flow(&mut app, F::ChangePlan);
    flow_into_vault(&mut shell, &mut app, 0, Thing::String(0, ix('d')));
    let sealed = round_and_write(&mut shell, &mut app, &[D]);
    assert_eq!(
        sealed_strings(&sealed, FLOW_PHRASE),
        vec![D.as_bytes().to_vec()],
        "the sealed copy holds D"
    );
}

/// A codex32 string moved out of a vault after the lock, another put in,
/// leaves it: the sealed copy holds the new one alone.
#[test]
fn a_codex32_string_moved_out_of_a_vault_after_the_lock_leaves_it() {
    let mut shell = Shell::new();
    let mut app = locked_with(&mut shell, &['d']);
    shell.flow(&mut app, F::ChangePlan);
    let z = flow_vault_zone(&app, 0);
    flow_board(
        &mut shell,
        &mut app,
        BoardPress::Off(z, Thing::String(0, ix('d'))),
    );
    flow_into_vault(&mut shell, &mut app, 0, Thing::String(0, ix('a')));
    let sealed = round_and_write(&mut shell, &mut app, &[A]);
    assert_eq!(
        sealed_strings(&sealed, FLOW_PHRASE),
        vec![A.as_bytes().to_vec()],
        "A in, D out"
    );
}

/// After a new set is made for the seed (Rotate the set), the vault keeps
/// the new set's string and not the old set's: Back up's vault item is
/// not done until it is saved again, and the vault's string with a paper
/// string of the new set gives vector 2's seed.
#[test]
fn after_a_new_set_the_vault_holds_the_new_sets_string_and_not_the_old_one() {
    assert!(BIP93.contains(SECRET));
    let c = testkit::compute();
    let mut app = device(vec![("home.ofv".into(), vault_of(PHRASE, Vec::new(), 7))]);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x70 + i; 32])));
    }
    unlock_on_vaults(&mut app, "home.ofv", PHRASE);
    key_from_strings(&mut app);
    wallet_and_backup(&mut app);
    vault_at(&mut app, 0, 0, &[]);
    into_vault(&mut app, 0, Thing::String(0, ix('d')));
    codex32_item(&mut app);
    type_each_back(&mut app);
    app.press(Action::BVaultSave(0));
    assert_eq!(
        share_records(&app, 0),
        vec![(D.as_bytes().to_vec(), FP.to_vec())]
    );
    assert!(app.backup_item_done_now(&c, plan::Item::Vault(0)));
    // Rotate the set: every old string held and destroyed, then a new set
    // of 2 of 3 named w0za.
    open_item(&mut app, bstep::of(plan::Item::Codex32(0)));
    let c32 = |app: &mut Faraday, p: Press| app.press(Action::BC32(p));
    c32(&mut app, Press::Set(SetPress::Rotate));
    c32(&mut app, Press::Set(SetPress::Counted));
    for j in ['a', 'c', 'd'] {
        c32(&mut app, Press::Set(SetPress::Destroyed(ix(j) as u8)));
    }
    assert!(matches!(
        app.backup().unwrap().c32.as_ref().unwrap().set,
        SetStep::Scheme(2, 3)
    ));
    for ch in "w0za".chars() {
        c32(&mut app, Press::Key(OskPress::Char(ch)));
    }
    c32(&mut app, Press::Set(SetPress::MakeNew));
    let k = app.session.keys.position_of(FP).unwrap();
    let facts = app.session.keys[k].codex32.clone().unwrap();
    assert_eq!(&facts.identifier, b"w0za", "the new set");
    let st = app.backup().unwrap().c32.as_ref().unwrap();
    let new: Vec<String> = facts
        .strings
        .iter()
        .filter_map(|&(ch, _)| st.made(plan::codex32::index_of(ch)?).map(str::to_string))
        .collect();
    assert_eq!(new.len(), 3);
    let in_vault = backup_held(&app, At::Vault(0))
        .into_iter()
        .find_map(|w| match w {
            What::Codex32Share(0, j) => Some(j),
            _ => None,
        })
        .expect("the plan keeps a string of the new set in the vault");
    let vault_string = new
        .iter()
        .find(|s| plan::codex32::index_of(s.as_bytes()[8]) == Some(in_vault))
        .unwrap()
        .clone();
    // The vault still holds the old set's D: not done.
    assert!(
        !app.backup_item_done_now(&c, plan::Item::Vault(0)),
        "a string of the old set is not the plan's"
    );
    app.press(Action::BVaultSave(0));
    assert_eq!(
        share_records(&app, 0),
        vec![(vault_string.as_bytes().to_vec(), FP.to_vec())],
        "the new set's string, and not the old set's"
    );
    assert!(app.backup_item_done_now(&c, plan::Item::Vault(0)));
    // The vault's string and another of the new set's give the seed back.
    let paper = new.iter().find(|s| **s != vault_string).unwrap();
    let parts = [
        Codex32::parse(&vault_string).unwrap(),
        Codex32::parse(paper).unwrap(),
    ];
    let secret = Codex32::recover(&parts).unwrap();
    let seed = secret.to_seed().unwrap();
    let want = Codex32::parse(SECRET).unwrap().to_seed().unwrap();
    assert_eq!(
        seed.expose().bytes(),
        want.expose().bytes(),
        "vector 2's seed"
    );
    assert_eq!(&secret.identifier(), b"w0za");
}
