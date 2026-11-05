//! The Keep ticks of a wallet's backup flow (`docs/VAULT.md` §7): a seed
//! copied into a vault of the plan asks which of its facts to keep.

use crate::backup_flow_wallet::{
    PHRASE, PHRASE_2, flow_vault_add, key_in, restart, two_of_three_ready, two_of_three_ready_with,
};
use crate::common::*;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Sheet};
use faraday_vault::records::kind;
use std::cell::Cell;

thread_local! {
    /// The second flow test's seed 1 is made a BIP-85 child before its save.
    pub(crate) static WANTED: Cell<bool> = const { Cell::new(false) };
    /// The plan vault and the texts of each Keep sheet [`save`] met.
    static SHEETS: std::cell::RefCell<Vec<(u8, Vec<String>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// The three keys of the wallet being made, by place; the first marked as
/// a BIP-85 child when [`WANTED`].
pub(crate) fn fps(app: &mut Faraday) -> Vec<[u8; 4]> {
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(app, i)).collect();
    if WANTED.get() {
        let k = app
            .session
            .keys
            .all()
            .position(|k| k.fingerprint.0 == fps[0])
            .unwrap();
        let of = faraday_vault::child::ChildOf {
            parent: [1, 2, 3, 4],
            language: 0,
            words: 24,
            index: 7,
        };
        assert!(app.session.keys.set_child_of(k, Some(of)));
    }
    fps
}

/// Vault `v` of the plan saved, its Keep sheet, if it asks, left as it is
/// and saved.
pub(crate) fn save(app: &mut Faraday, v: u8) {
    app.press(Action::BVaultSave(v));
    if app.sheet == Some(Sheet::SaveKeeps) {
        let shown = texts(app);
        SHEETS.with_borrow_mut(|l| l.push((v, shown)));
        app.press(Action::Vault(V::KeepSave));
    }
}

/// How many key records of the open vaults carry a BIP-85 origin.
fn origins_in_open(app: &Faraday) -> usize {
    app.vaults()
        .open
        .iter()
        .map(|o| {
            app.vaults()
                .store()
                .contents_for_tests(o.id)
                .of(kind::KEY)
                .filter(|(_, r)| {
                    r.fields
                        .iter()
                        .any(|f| f.number == faraday_vault::records::field::KEY_CHILD)
                })
                .count()
        })
        .sum()
}

#[test]
fn a_seed_copied_from_a_holder_into_an_added_vault_asks_and_unticked_arrives_without_its_origin() {
    let mut shell = Shell::new();
    WANTED.set(true);
    let mut app = two_of_three_ready(&mut shell, false);
    // The vault the plan wrote the seed into holds its origin.
    assert_eq!(origins_in_open(&app), 1);
    let fp = app
        .session
        .keys
        .all()
        .find(|k| k.child_of.is_some())
        .unwrap()
        .fingerprint
        .0;
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    flow_vault_add(&mut shell, &mut app, 1, 0);
    shell.flow(&mut app, F::Continue);
    open_vault(&mut shell, &mut app, 0, PHRASE);
    open_vault(&mut shell, &mut app, 1, PHRASE_2);
    shell.flow(&mut app, F::CopyKey);
    assert_eq!(app.sheet, Some(Sheet::SaveKeeps), "{:?}", app.toast_text());
    assert_eq!(origins_in_open(&app), 1, "written before the question");
    app.press(Action::Vault(V::KeepTick(
        fp,
        faraday_vault::records::field::KEY_CHILD,
    )));
    app.press(Action::Vault(V::KeepSave));
    assert_eq!(app.sheet, None);
    assert_eq!(origins_in_open(&app), 1, "the copy arrived without it");
}

#[test]
fn an_untick_and_the_asked_mark_in_one_flow_do_not_reach_another() {
    let mut shell = Shell::new();
    WANTED.set(true);
    let mut app = two_of_three_ready(&mut shell, false);
    let fp = app
        .session
        .keys
        .all()
        .find(|k| k.child_of.is_some())
        .unwrap()
        .fingerprint
        .0;
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    flow_vault_add(&mut shell, &mut app, 1, 0);
    shell.flow(&mut app, F::Continue);
    open_vault(&mut shell, &mut app, 0, PHRASE);
    open_vault(&mut shell, &mut app, 1, PHRASE_2);
    shell.flow(&mut app, F::CopyKey);
    assert_eq!(app.sheet, Some(Sheet::SaveKeeps), "{:?}", app.toast_text());
    // A second flow, as the first is before anything is ticked.
    let second = faraday_core::testkit::flows_mut(&mut app)[0].clone();
    faraday_core::testkit::flows_mut(&mut app).push(second);
    app.press(Action::Vault(V::KeepTick(
        fp,
        faraday_vault::records::field::KEY_CHILD,
    )));
    app.press(Action::Vault(V::KeepSave));
    assert_eq!(origins_in_open(&app), 1, "the first flow's copy lacks it");
    // The entry the first flow wrote of that seed, the one without the
    // origin, is taken out, so the second has a key to copy.
    let c = faraday_core::testkit::compute();
    let ids: Vec<_> = app.vaults().open.iter().map(|o| o.id).collect();
    for id in ids {
        faraday_core::testkit::vault_edit(&mut app, id, |s| {
            s.remove_where(id, |r| {
                r.kind() == kind::KEY
                    && !r.has(faraday_vault::records::field::KEY_CHILD)
                    && faraday_keys::vault::key_identity(&c, faraday_core::testkit::NET, r)
                        .is_some_and(|(f, _)| f.0 == fp)
            })
        });
    }
    shell.flow(&mut app, F::Show(1));
    shell.flow(&mut app, F::CopyKey);
    assert_eq!(
        app.sheet,
        Some(Sheet::SaveKeeps),
        "the second flow asks: {:?}",
        app.toast_text()
    );
    app.press(Action::Vault(V::KeepSave));
    assert_eq!(app.sheet, None);
    assert_eq!(origins_in_open(&app), 2, "ticked, the copy has its origin");
}

#[test]
fn an_unticked_origin_is_left_out_when_the_plan_keeps_the_passphrase_apart() {
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    let mut app = two_of_three_ready_with(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    flow_vault_add(&mut shell, &mut app, 1, 0);
    shell.flow(&mut app, F::Continue);
    open_vault(&mut shell, &mut app, 0, PHRASE);
    open_vault(&mut shell, &mut app, 1, PHRASE_2);
    let has = |app: &Faraday, o: usize, n: u8| {
        let id = app.vaults().open[o].id;
        app.vaults()
            .store()
            .contents_for_tests(id)
            .of(kind::KEY)
            .any(|(_, r)| r.fields.iter().any(|f| f.number == n))
    };
    // The holder's entry for the seed carries its passphrase too.
    let id = app.vaults().open[0].id;
    let (at, fields): (usize, Vec<(u8, Vec<u8>)>) = app
        .vaults()
        .store()
        .contents_for_tests(id)
        .of(kind::KEY)
        .next()
        .map(|(k, r)| {
            (
                k,
                r.fields
                    .iter()
                    .map(|f| (f.number, f.bytes.to_vec()))
                    .collect(),
            )
        })
        .expect("the holder has the seed");
    let mut with = faraday_vault::Record::new(kind::KEY);
    for (n, b) in &fields {
        with = with.with(*n, b);
    }
    with = with.with(field::KEY_PASSPHRASE, b"kept apart").with(
        field::KEY_CHILD,
        &[1, 2, 3, 4, 39, 0, 0, 0, 0, 24, 0, 0, 0, 0],
    );
    faraday_core::testkit::vault_edit(&mut app, id, |s| {
        s.remove(id, at);
        s.insert(id, with);
    });
    assert!(has(&app, 0, field::KEY_PASSPHRASE), "the holder keeps it");
    assert!(has(&app, 0, field::KEY_CHILD), "the holder has the origin");
    shell.flow(&mut app, F::CopyKey);
    assert_eq!(app.sheet, Some(Sheet::SaveKeeps), "{:?}", app.toast_text());
    let fps = faraday_core::testkit::keep_sheet_seeds(&app);
    assert!(!fps.is_empty());
    for fp in fps {
        app.press(Action::Vault(V::KeepTick(fp, field::KEY_CHILD)));
    }
    app.press(Action::Vault(V::KeepSave));
    assert_eq!(app.sheet, None);
    assert!(!has(&app, 1, field::KEY_CHILD), "the origin was written");
    assert!(
        !has(&app, 1, field::KEY_PASSPHRASE),
        "the passphrase was written"
    );
}

#[test]
fn the_keep_sheet_of_a_backups_save_names_the_vault_the_seed_goes_into() {
    let mut shell = Shell::new();
    WANTED.set(true);
    SHEETS.with_borrow_mut(Vec::clear);
    let app = two_of_three_ready(&mut shell, false);
    let sheets = SHEETS.with_borrow(Clone::clone);
    assert!(!sheets.is_empty(), "no Keep sheet came up");
    for (v, shown) in sheets {
        let o = &app.vaults().open[usize::from(v)];
        let want = format!("Save into {}", o.label(app.vaults().store()));
        assert!(shown.contains(&want), "{want}: {shown:?}");
    }
}
