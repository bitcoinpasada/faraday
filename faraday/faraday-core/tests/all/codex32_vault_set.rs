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
    flow_vault_zone, kdbx, restart, save_vaults, step, sticks, write_sticks,
};
use crate::codex32_kept::{
    A, C, D, FP, PHRASE, codex32_item, ix, key_from_strings, share_record, share_records,
    type_each_back, unlock_on_vaults, vault_of, wallet_and_backup,
};
use crate::common::*;
use faraday_bip::codex32::Codex32;
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::board::{BoardPress, Thing, Zone};
use faraday_core::codex32_backup::{Press, SetPress, SetStep};
use faraday_core::plan::{self, At, What};
use faraday_core::vault::VaultAction as VA;
use faraday_core::{Action, Faraday, bstep, cstep, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_ui::keyboard::OskPress;
use faraday_vault::records::{field, kind};

pub(crate) const BIP93: &str = include_str!("../../../../tools/vectors/bip93/vectors.json");
/// Vector 2's secret, the seed the set splits.
pub(crate) const SECRET: &str = "MS12NAMES6XQGUZTTXKEQNJSJZV4JV3NZ5K3KWGSPHUH6EVW";

/// The codex32 strings `bytes`, a vault file, holds once opened with
/// `phrase`.
pub(crate) fn sealed_strings(bytes: &[u8], phrase: &str) -> Vec<Vec<u8>> {
    let c = testkit::compute();
    let mut store = faraday_keys::vault::OpenVaults::default();
    let (id, _) = store.unlock(&c, bytes.to_vec(), phrase).unwrap();
    store
        .contents_for_tests(id)
        .iter()
        .filter(|r| r.kind == kind::CODEX32_SHARE)
        .map(|r| r.field(field::CODEX32_STRING).unwrap_or_default().to_vec())
        .collect()
}

/// A single-key wallet over vector 2's key, typed in from strings A and C
/// inside Make a new wallet with one vault on stick S1, the plan as it
/// opens (the seed and the description in that vault, a string on paper
/// at each place): the checklist done with `in_vault`'s
/// strings in the vault, locked, and stopped on the writes.
pub(crate) fn locked_with(shell: &mut Shell, in_vault: &[char]) -> Faraday {
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
    assert_eq!(
        app.session.keys.keep_codex32(&testkit::compute(), k, typed),
        typed.len()
    );
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
    kdbx(shell, "S1").expect("Vault 1 on S1")
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

/// A codex32 string taken out of the plan after the lock, another put in:
/// the sealed copy holds the new one, and the old stays, as the plan lists
/// it in no vault (owner, 2026-11-05).
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
    let mut held = sealed_strings(&sealed, FLOW_PHRASE);
    held.sort();
    assert_eq!(
        held,
        vec![A.as_bytes().to_vec(), D.as_bytes().to_vec()],
        "A in, D stays"
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
    let mut app = device(vec![("home.kdbx".into(), vault_of(PHRASE, Vec::new(), 7))]);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x70 + i; 32])));
    }
    unlock_on_vaults(&mut app, "home.kdbx", PHRASE);
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

/// Back up's board over vector 2's key with its string D in vault 0, which
/// holds `extra` too, saved once.
fn saved_with_d(extra: Vec<faraday_vault::Record>) -> Faraday {
    let mut records = vec![share_record(D)];
    records.extend(extra);
    let mut app = device(vec![("home.kdbx".into(), vault_of(PHRASE, records, 7))]);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x70 + i; 32])));
    }
    unlock_on_vaults(&mut app, "home.kdbx", PHRASE);
    key_from_strings(&mut app);
    wallet_and_backup(&mut app);
    vault_at(&mut app, 0, 0, &[]);
    into_vault(&mut app, 0, Thing::String(0, ix('d')));
    codex32_item(&mut app);
    type_each_back(&mut app);
    save_keeping(&mut app);
    app
}

/// Vault 0 saved from Back up; a vault that holds a share of the key
/// already asks first, and the person goes on.
fn save_keeping(app: &mut Faraday) {
    app.press(Action::BVaultSave(0));
    if app.sheet == Some(faraday_core::Sheet::SaveAlso) {
        app.press(Action::Vault(VA::SaveAlsoContinue));
    }
}

/// The set rotated to a new one of 2 of 3 named `id`, every string of the
/// old set held and destroyed. The new set's strings.
fn rotate_to(app: &mut Faraday, id: &str) -> Vec<String> {
    try_rotate(app, id);
    let k = app.session.keys.position_of(FP).unwrap();
    let facts = app.session.keys[k].codex32.clone().unwrap();
    assert_eq!(
        facts.identifier,
        <[u8; 4]>::try_from(id.as_bytes()).unwrap()
    );
    let st = app.backup().unwrap().c32.as_ref().unwrap();
    facts
        .strings
        .iter()
        .filter_map(|&(ch, _)| st.made(plan::codex32::index_of(ch)?).map(str::to_string))
        .collect()
}

/// The presses of a rotation to a new set named `id`, as far as the
/// screen lets them go.
fn try_rotate(app: &mut Faraday, id: &str) {
    let k = app.session.keys.position_of(FP).unwrap();
    let old: Vec<u8> = app.session.keys[k]
        .codex32
        .clone()
        .unwrap()
        .strings
        .iter()
        .map(|&(ch, _)| ch)
        .collect();
    open_item(app, bstep::of(plan::Item::Codex32(0)));
    let c32 = |app: &mut Faraday, p: Press| app.press(Action::BC32(p));
    c32(app, Press::Set(SetPress::Rotate));
    c32(app, Press::Set(SetPress::Counted));
    for ch in old {
        let j = plan::codex32::index_of(ch).unwrap();
        c32(app, Press::Set(SetPress::Destroyed(j as u8)));
    }
    for ch in id.chars() {
        c32(app, Press::Key(OskPress::Char(ch)));
    }
    c32(app, Press::Set(SetPress::MakeNew));
}

/// Each of `strings` typed back from its paper in the codex32 item.
fn type_back(app: &mut Faraday, strings: &[String]) {
    open_item(app, bstep::of(plan::Item::Codex32(0)));
    for s in strings {
        let j = plan::codex32::index_of(s.as_bytes()[8]).unwrap();
        app.press(Action::BC32(Press::Open(j as u8)));
        for ch in s[3..].chars() {
            app.press(Action::BC32(Press::Key(OskPress::Char(ch))));
        }
        app.press(Action::BC32(Press::Next));
    }
}

fn strings_held(app: &Faraday) -> Vec<Vec<u8>> {
    let mut held: Vec<Vec<u8>> = share_records(app, 0).into_iter().map(|(s, _)| s).collect();
    held.sort();
    held
}

/// The plan remembers the set rotated away: a string of it that comes
/// back into the vault after the plan was saved goes at the next save,
/// and the plan opened again from the vault still knows.
#[test]
fn a_string_of_a_set_rotated_away_goes_at_a_save_after_the_plan_is_reopened() {
    let c = testkit::compute();
    let mut app = saved_with_d(Vec::new());
    rotate_to(&mut app, "w0za");
    save_keeping(&mut app);
    assert!(!strings_held(&app).contains(&D.as_bytes().to_vec()));
    // D comes back into the vault (a copy from elsewhere); Back up is left
    // and opened again, its plan read from the vault.
    testkit::vault_add_amounts(&mut app, 0, vec![share_record(D)]);
    app.press(Action::Nav(faraday_core::Screen::Home));
    app.press(Action::Backup(app.session.wallets.len() - 1));
    assert!(
        !app.backup_item_done_now(&c, plan::Item::Vault(0)),
        "the old set's string is not the plan's"
    );
    save_keeping(&mut app);
    assert!(!strings_held(&app).contains(&D.as_bytes().to_vec()));
    assert!(app.backup_item_done_now(&c, plan::Item::Vault(0)));
}

/// A second set of the seed that the plan never made, moved in, stays at
/// a save after the plan's own set was rotated away.
#[test]
fn a_string_of_a_set_the_plan_never_made_stays_after_a_rotation() {
    // Another set of the key, as the flow makes one.
    let mut scratch = saved_with_d(Vec::new());
    let other = rotate_to(&mut scratch, "w0za")[0].clone();
    let mut app = saved_with_d(vec![share_record(&other)]);
    let new = rotate_to(&mut app, "w2za");
    save_keeping(&mut app);
    let held = strings_held(&app);
    assert!(
        held.contains(&other.as_bytes().to_vec()),
        "w0za was taken out"
    );
    assert!(!held.contains(&D.as_bytes().to_vec()), "D stayed");
    assert!(held.iter().any(|h| new.iter().any(|n| n.as_bytes() == h)));
}

/// A set rotated twice: strings of both old sets go at a save.
#[test]
fn strings_of_both_sets_rotated_away_go_at_a_save() {
    let mut app = saved_with_d(Vec::new());
    let first = rotate_to(&mut app, "w0za");
    save_keeping(&mut app);
    type_back(&mut app, &first);
    // The second set draws its own randomness.
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x90 + i; 32])));
    }
    let second = rotate_to(&mut app, "w2za");
    // A string of each old set is in the vault.
    testkit::vault_add_amounts(&mut app, 0, vec![share_record(D), share_record(&first[0])]);
    save_keeping(&mut app);
    let held = strings_held(&app);
    assert!(
        !held.contains(&D.as_bytes().to_vec()),
        "the first set stayed"
    );
    for s in &first {
        assert!(
            !held.contains(&s.as_bytes().to_vec()),
            "the second set stayed"
        );
    }
    assert!(
        held.iter()
            .any(|h| second.iter().any(|n| n.as_bytes() == h))
    );
}

/// A key whose codex32 set is known, saved on Vaults with the set's tick
/// left out, is in the vault as its words alone, and so after a lock and
/// an unlock; the session still holds the set.
#[test]
fn a_key_saved_on_vaults_with_the_set_unticked_has_no_set_in_its_entry_after_a_lock() {
    let entries = |app: &Faraday| {
        let id = app.vaults().open[0].id;
        app.vaults()
            .records(id)
            .of(kind::KEY)
            .map(|(_, r)| (r.has(field::KEY), r.has(field::KEY_CODEX32)))
            .collect::<Vec<_>>()
    };
    let mut app = device(vec![("home.kdbx".into(), vault_of(PHRASE, Vec::new(), 7))]);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x70 + i; 32])));
    }
    unlock_on_vaults(&mut app, "home.kdbx", PHRASE);
    key_from_strings(&mut app);
    let k = app.session.keys.position_of(FP).unwrap();
    app.press(Action::Vault(VA::SaveKey(k)));
    assert_eq!(app.sheet, Some(faraday_core::Sheet::SaveKeeps));
    app.press(Action::Vault(VA::KeepTick(FP, field::KEY_CODEX32)));
    app.press(Action::Vault(VA::KeepSave));
    assert_eq!(app.sheet, None);
    assert_eq!(entries(&app), vec![(true, false)], "the words alone");
    assert!(app.session.keys[k].codex32.is_some(), "the session's set");
    app.press(Action::Lock);
    testkit::settle(&mut app);
    let sealed = app
        .outbox
        .iter()
        .find(|i| i.name == "home.kdbx")
        .map(|i| (String::from(&*i.name), i.bytes.to_vec()))
        .expect("the lock sealed the vault into the Outbox");
    let mut next = device(vec![sealed]);
    unlock_on_vaults(&mut next, "home.kdbx", PHRASE);
    assert_eq!(entries(&next), vec![(true, false)], "after the lock");
}

/// A plan text over one key whose set `name` is on the plan, with `extra`
/// lines after it.
fn plan_text_with(extra: &str, before: bool) -> Option<plan::Answers> {
    let shape = plan::Shape {
        m: 1,
        keys: 1,
        seeds: vec![plan::Seed {
            name: faraday_text::Text::of("aaaa0001"),
            here: true,
            passphrase: false,
        }],
        splits: false,
        vaults: 0,
    };
    let base = String::from(&*plan::Answers::preset(&shape, plan::Preset::Standard).to_text());
    let set = "codex32 0 2 48 ac name 0\n";
    let text = if before {
        format!("{base}{extra}{set}")
    } else {
        format!("{base}{set}{extra}")
    };
    plan::Answers::from_text(&shape, &text)
}

/// A vault's plan is untrusted: a line naming the plan's own set as
/// rotated away would take the plan's strings out at a save, and an
/// identifier that is not bech32 would not read again once saved.
#[test]
fn a_plan_read_from_a_vault_refuses_a_rotated_line_that_cannot_be_one() {
    for before in [false, true] {
        let ok = plan_text_with("rotated 0 2 w0za\n", before).expect("a real rotated set reads");
        assert_eq!(ok.rotated, vec![(0, 2, *b"w0za")]);
        for line in [
            "rotated 0 2 name\n",
            "rotated 0 3 name\n",
            "rotated 0 2 NAME\n",
            "rotated 0 2 n\u{e9}\u{e9}\n",
            "rotated 0 2 NAMX\n",
            "rotated 0 2 bbbb\n",
        ] {
            assert!(plan_text_with(line, before).is_none(), "{line:?} read");
        }
    }
}

/// The key loaded from papers of another set of the seed than the plan's
/// form: the plan takes that set as its form and records nothing, so the
/// plan's own strings stay at the next save.
#[test]
fn a_key_loaded_with_another_set_than_the_plans_does_not_take_the_plans_strings_out() {
    let mut scratch = saved_with_d(Vec::new());
    rotate_to(&mut scratch, "w0za");
    let k = scratch.session.keys.position_of(FP).unwrap();
    let other = scratch.session.keys[k].codex32.clone().unwrap();
    let mut app = saved_with_d(Vec::new());
    let k = app.session.keys.position_of(FP).unwrap();
    app.session.keys.set_codex32(k, Some(other));
    app.press(Action::Nav(faraday_core::Screen::Home));
    app.press(Action::Backup(app.session.wallets.len() - 1));
    save_keeping(&mut app);
    assert!(
        strings_held(&app).contains(&D.as_bytes().to_vec()),
        "D was taken out"
    );
}

/// A set rotated away, its papers loaded as the key again while the
/// plan still has the new set: the new set stays the plan's, and its
/// strings stay at a save.
#[test]
fn the_old_sets_papers_loaded_again_do_not_make_the_new_sets_strings_go() {
    let mut app = saved_with_d(Vec::new());
    let k = app.session.keys.position_of(FP).unwrap();
    let old = app.session.keys[k].codex32.clone().unwrap();
    let new = rotate_to(&mut app, "w0za");
    save_keeping(&mut app);
    app.session.keys.set_codex32(k, Some(old));
    app.press(Action::Nav(faraday_core::Screen::Home));
    app.press(Action::Backup(app.session.wallets.len() - 1));
    save_keeping(&mut app);
    let held = strings_held(&app);
    assert!(
        held.iter().any(|h| new.iter().any(|n| n.as_bytes() == h)),
        "the new set's strings went"
    );
    assert!(!held.contains(&D.as_bytes().to_vec()), "D came back");
}

/// A rotation back to an identifier the seed had before is refused, as
/// the current one is: the old set's papers were destroyed.
#[test]
fn a_rotation_back_to_an_identifier_rotated_away_is_refused() {
    let mut app = saved_with_d(Vec::new());
    let k = app.session.keys.position_of(FP).unwrap();
    let first = app.session.keys[k].codex32.clone().unwrap().identifier;
    let first_new = rotate_to(&mut app, "w0za");
    save_keeping(&mut app);
    type_back(&mut app, &first_new);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x90 + i; 32])));
    }
    let id: String = first.iter().map(|&b| char::from(b)).collect();
    try_rotate(&mut app, &id);
    let facts = app.session.keys[k].codex32.clone().unwrap();
    assert_eq!(&facts.identifier, b"w0za", "the old identifier was taken");
    assert!(
        app.backup()
            .unwrap()
            .c32
            .as_ref()
            .is_some_and(|st| st.note.is_some()),
        "no refusal shown"
    );
}

/// The old set's papers loaded as the key, with D, while the plan has the
/// new set rotated to and saved: the key as `saved_with_d` left it, then
/// its facts the old set's and the old strings held, and Back up reopened.
fn old_papers_loaded_over_a_rotation() -> (Faraday, Vec<String>) {
    let mut app = saved_with_d(Vec::new());
    let k = app.session.keys.position_of(FP).unwrap();
    let old = app.session.keys[k].codex32.clone().unwrap();
    let new = rotate_to(&mut app, "w0za");
    save_keeping(&mut app);
    app.session.keys.set_codex32(k, Some(old));
    app.session
        .keys
        .keep_codex32(&testkit::compute(), k, &[A, C, D]);
    assert!(app.session.keys.holds_codex32(k, b'd'), "the old D is held");
    app.press(Action::Nav(faraday_core::Screen::Home));
    app.press(Action::Backup(app.session.wallets.len() - 1));
    (app, new)
}

/// A string of the set rotated away, held by the session again, is not
/// written back into a vault as the plan's string.
#[test]
fn a_string_of_a_set_rotated_away_is_not_written_back_at_a_save() {
    let (mut app, _) = old_papers_loaded_over_a_rotation();
    save_keeping(&mut app);
    assert!(
        !strings_held(&app).contains(&D.as_bytes().to_vec()),
        "D came back"
    );
}

/// The new set's D is in the first vault and the plan moves it to a vault
/// that is closed; the session holds the old set's D, which is not it: the
/// save of the first vault keeps the only new D.
#[test]
fn an_old_sets_string_in_the_session_does_not_let_the_new_sets_share_go() {
    let (mut app, new) = old_papers_loaded_over_a_rotation();
    let new_d = new
        .iter()
        .find(|s| s.as_bytes()[8] == b'd')
        .unwrap()
        .clone();
    assert!(strings_held(&app).contains(&new_d.as_bytes().to_vec()));
    vault_at(&mut app, 0, 1, &[]);
    into_vault(&mut app, 1, Thing::String(0, ix('d')));
    let p = 0;
    let s = vault_stick(&app, p, 0).unwrap();
    take_off(&mut app, Zone::Vault(p, s, 0), Thing::String(0, ix('d')));
    save_keeping(&mut app);
    assert!(
        strings_held(&app).contains(&new_d.as_bytes().to_vec()),
        "the only new D was taken out"
    );
}

/// A string the plan keeps on a stick, the session holding only the old
/// set's: Lock and write names the string as missing.
#[test]
fn an_old_sets_string_does_not_stand_for_the_plans_on_a_stick() {
    let (mut app, _) = old_papers_loaded_over_a_rotation();
    put(&mut app, Thing::String(0, ix('d')), Zone::NewStick(0));
    let c = testkit::compute();
    assert!(
        app.backup_stick_string_missing(&c).is_some(),
        "the old D stands for the new one"
    );
}
