//! The backup board's model (`docs/BACKUP-FLOW.md` §19, F37), before the
//! board is drawn: sticks at a place (`At::Stick`), each with at most one
//! vault and files beside it; a vault one vault wherever its sticks are;
//! the files made and written with their stick, under their own names; a
//! seed kept nowhere holding the plan, and a seed kept only on a stick
//! held until that stick is written (F31); a plan saved before the sticks
//! read as it was. Driven through Back up's cards and the flow where they
//! exist, through the plan's own edits where the board will be.

mod common;

use common::*;
use faraday_core::backup_flow::{BackupFlow, Step, Target};
use faraday_core::backup_flow_steps::FlowAction as F;
use faraday_core::create::NewKind;
use faraday_core::plan::{
    self, Answers, At, Form, Item, Preset, Row, Seed, Shape, Spot, What, checklist, form,
    kept_nowhere,
};
use faraday_core::testkit;
use faraday_core::{Action, BStage, Faraday, Screen, Sheet, bstep, cstep, qstep};
use osk_bip::keys::Fingerprint;
use osk_shell_api::{App, Event, Key};

const PHRASE: &str = "first vault phrase";

// ---- helpers ----

/// `n` empty sticks of different serials.
fn sticks(shell: &mut Shell, n: usize) {
    for k in 1..=n {
        let mut d = Disk::empty(&format!("S{k}"), "KEYSTICK");
        d.serial = Some(k as u32);
        shell.add(d);
    }
}

/// Test seed `n` typed in as a key.
fn key_in(app: &mut Faraday, n: usize) -> [u8; 4] {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.session.keys.last().unwrap().fingerprint().0
}

fn multi_index() -> u8 {
    NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8
}

fn backup_places(app: &Faraday) -> usize {
    app.backup.as_ref().unwrap().answers.places
}

fn backup_shape(app: &Faraday) -> Shape {
    app.plan_shape(app.backup.as_ref().unwrap().wallet)
}

/// The plan's map as it is, sticks and all.
fn the_map(app: &Faraday) -> Vec<Spot> {
    plan::map(&backup_shape(app), &app.backup.as_ref().unwrap().answers)
}

/// What spot `at` holds on the map `m`.
fn held(m: &[Spot], at: At) -> Vec<What> {
    m.iter()
        .filter(|s| s.at == at)
        .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
        .collect()
}

/// A change the board will make, made to Back up's plan through the
/// plan's own edits (the board is not drawn yet).
fn board_edit(app: &mut Faraday, f: impl FnOnce(Vec<Spot>) -> Vec<Spot>) {
    let shape = backup_shape(app);
    let a = &mut app.backup.as_mut().unwrap().answers;
    let m = plan::map(&shape, a);
    a.set_map(f(m));
}

/// Row `row` unticked on every place's card of Back up's plan.
fn untick_everywhere(app: &mut Faraday, row: Row) {
    for p in 0..backup_places(app) {
        app.press(Action::BRowOff(p as u8, row.code()));
    }
}

/// The description's form `form` unticked at every place that keeps it.
fn form_off_everywhere(app: &mut Faraday, form: Form) {
    for p in 0..backup_places(app) {
        let shape = backup_shape(app);
        if app
            .backup
            .as_ref()
            .unwrap()
            .answers
            .form_at(&shape, p, form)
        {
            app.press(Action::BForm(p as u8, form.code()));
        }
    }
}

/// At place `p`, row `row` given exactly `seeds` on its sheet, and Done.
fn keep_at(app: &mut Faraday, p: usize, row: Row, seeds: &[usize]) {
    if app.backup.as_ref().unwrap().q != Some(qstep::place(p)) {
        app.press(Action::BQ(qstep::place(p)));
    }
    app.press(Action::BRow(p as u8, row.code()));
    let ticked = app
        .choosing
        .as_ref()
        .expect("the row's sheet")
        .seeds
        .clone();
    for i in 0..backup_shape(app).seeds.len() {
        if ticked.contains(&i) != seeds.contains(&i) {
            app.press(Action::BPick(i as u8));
        }
    }
    app.press(Action::BPickDone);
}

/// A 2-of-3 made in Create over three keys typed in here, its backup
/// open on Paper only: three places.
fn three_here() -> Faraday {
    let mut app = testkit::started();
    for n in 0..3 {
        key_in(&mut app, n);
    }
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    for slot in 0..3u8 {
        let fp = app.session.keys[usize::from(slot)].fingerprint().0;
        app.press(Action::CSlotHere(slot, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(0));
    assert_eq!(backup_places(&app), 3);
    app
}

/// Saves into the open vaults what the plan puts in each.
fn save_vaults(app: &mut Faraday) {
    let vaults: Vec<usize> = app
        .backup_items()
        .into_iter()
        .filter_map(|n| match bstep::item(n) {
            Some(Item::Vault(v)) => Some(v),
            _ => None,
        })
        .collect();
    for v in vaults {
        app.press(Action::BVaultSave(v as u8));
        assert!(
            app.backup_item_done(Item::Vault(v)),
            "Vault {} saved",
            v + 1
        );
    }
}

fn restart(shell: &mut Shell, app: &mut Faraday) -> Faraday {
    shell.pump(app);
    assert!(app.restart_requested(), "locked");
    shell.restart()
}

fn step(app: &Faraday) -> Step {
    app.flow().expect("a flow").step
}

fn names(shell: &Shell, d: &str) -> Vec<String> {
    shell.disk(d).files.iter().map(|f| f.0.clone()).collect()
}

/// Which stick pills are lit, by number: the sticks in that the flow
/// recognises as its own (§18).
fn lit(app: &Faraday) -> Vec<usize> {
    app.stick_pills()
        .iter()
        .filter(|p| p.inserted)
        .map(|p| p.n)
        .collect()
}

/// The sticks a flow writes at places, in order: the stick of public
/// files, which the sheets on paper go out on, aside.
fn places_targets(flow: &BackupFlow) -> Vec<Target> {
    flow.slots
        .iter()
        .map(|s| s.target)
        .filter(|t| *t != Target::Public)
        .collect()
}

/// The name a seed's file is written as on a stick (§19.12 item 6).
fn stick_name(prefix: &str, fp: [u8; 4], ext: &str) -> String {
    format!(
        "{prefix}-{}.{ext}",
        faraday_core::wallet::fp_text(Fingerprint(fp)).to_lowercase()
    )
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

// ---- the tests ----

/// Vault 1 kept at Place 1 and at Place 3 is one vault: what it holds is
/// on the map once, in the vault, and each place keeps a stick of it.
#[test]
fn a_vault_at_two_places_is_held_once() {
    let mut app = three_here();
    keep_at(&mut app, 0, Row::Vault(0), &[0]);
    keep_at(&mut app, 2, Row::Vault(0), &[0, 1]);
    let m = the_map(&app);
    let vaults: Vec<&Spot> = m.iter().filter(|s| s.at == At::Vault(0)).collect();
    assert_eq!(vaults.len(), 1, "one vault: {m:?}");
    assert_eq!(held(&m, At::Vault(0)), vec![What::Seed(0), What::Seed(1)]);
    // Each seed in it is held there and nowhere else on the map.
    for i in [0, 1] {
        let at: Vec<At> = m
            .iter()
            .filter(|s| {
                s.holds
                    .iter()
                    .any(|(w, _)| matches!(w, What::Seed(j) | What::SeedPassphrase(j) if *j == i))
            })
            .map(|s| s.at)
            .collect();
        assert_eq!(at, vec![At::Vault(0)], "seed {i}");
    }
    assert_eq!(held(&m, At::Stick(0, 0)), vec![What::VaultStick(0)]);
    assert_eq!(held(&m, At::Stick(2, 0)), vec![What::VaultStick(0)]);
    assert!(held(&m, At::Place(0)).iter().all(|w| !plan::on_stick(*w)));
    // The write step writes a copy of it at each place.
    let shape = backup_shape(&app);
    let flow = BackupFlow::new_wallet(shape, app.backup.as_ref().unwrap().answers.clone(), "w");
    assert_eq!(
        places_targets(&flow),
        vec![Target::Place(0), Target::Place(2)]
    );
}

/// A second vault kept at a place goes on a stick of its own there; the
/// board's drop of a vault on a place where every stick carries one, its
/// own included, adds a stick; taking a vault off the plan takes its
/// sticks with it.
#[test]
fn a_second_vault_at_a_place_goes_on_a_second_stick() {
    let mut app = three_here();
    keep_at(&mut app, 0, Row::Vault(0), &[0]);
    keep_at(&mut app, 0, Row::Vault(1), &[1]);
    let m = the_map(&app);
    assert_eq!(held(&m, At::Stick(0, 0)), vec![What::VaultStick(0)]);
    assert_eq!(held(&m, At::Stick(0, 1)), vec![What::VaultStick(1)]);
    assert_eq!(plan::sticks_at(&m, 0), 2);
    // Written as two sticks, each its vault's.
    let shape = backup_shape(&app);
    let flow = BackupFlow::new_wallet(
        shape.clone(),
        app.backup.as_ref().unwrap().answers.clone(),
        "w",
    );
    assert_eq!(
        places_targets(&flow),
        vec![Target::PlaceVault(0, 0), Target::PlaceVault(0, 1)]
    );
    // The board's drop: Vault 1 again at Place 1 is a second stick of it
    // (§19.12 item 7).
    let a = &mut app.backup.as_mut().unwrap().answers;
    a.add_vault_stick(&shape, 0, 0);
    let m = plan::map(&shape, a);
    assert_eq!(plan::sticks_at(&m, 0), 3);
    assert_eq!(held(&m, At::Stick(0, 2)), vec![What::VaultStick(0)]);
    // A file dropped on Place 1 goes on a stick of files of its own; the
    // vault dropped again goes on that stick, its first with no vault.
    let m = plan::with_added(&m, At::Place(0), What::SeedFile(2));
    assert_eq!(held(&m, At::Stick(0, 3)), vec![What::SeedFile(2)]);
    let m = plan::with_vault_stick(&m, 0, 1);
    assert_eq!(
        held(&m, At::Stick(0, 3)),
        vec![What::SeedFile(2), What::VaultStick(1)]
    );
    a.set_map(m);
    // Vault 1 taken off the plan: its sticks go, the vault with them.
    a.remove_vault(&shape, 0);
    let m = plan::map(&shape, a);
    assert!(m.iter().all(|s| s.at != At::Vault(0)));
    assert!(
        m.iter()
            .all(|s| s.holds.iter().all(|(w, _)| *w != What::VaultStick(0)))
    );
    assert_eq!(held(&m, At::Stick(0, 0)), vec![What::VaultStick(1)]);
    assert_eq!(
        held(&m, At::Stick(0, 1)),
        vec![What::SeedFile(2), What::VaultStick(1)]
    );
}

/// Seed B's words file on the stick that carries Vault 1, which holds
/// Seed A: the write step writes the vault and the file in one visit to
/// that stick, and the stick put back afterwards is recognised as the one
/// written.
#[test]
fn seed_bs_words_file_beside_vault_1_is_written_in_one_visit_and_the_stick_is_recognised() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    // A 2-of-3 with one vault set up before its keys.
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CVaultN(1));
    app.press(Action::CNext(cstep::HOW_MANY));
    assert_eq!(step(&app), Step::Sticks);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    create_vault(&mut shell, &mut app, 0, PHRASE);
    assert_eq!(app.screen, Screen::Create, "{:?}", app.vaults.unlock_error);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(0));
    // Seed A in Vault 1 at Place 1; Seed C as a file at Place 2; nothing
    // on paper.
    untick_everywhere(&mut app, Row::Words);
    form_off_everywhere(&mut app, Form::Paper);
    keep_at(&mut app, 0, Row::Vault(0), &[0]);
    keep_at(&mut app, 1, Row::File, &[2]);
    // Seed B's words file dropped on Vault 1's stick at Place 1.
    board_edit(&mut app, |m| {
        plan::with_added(&m, At::Stick(0, 0), What::SeedFile(1))
    });
    let m = the_map(&app);
    assert_eq!(
        held(&m, At::Stick(0, 0)),
        vec![What::VaultStick(0), What::SeedFile(1)]
    );
    assert_eq!(app.backup_kept_nowhere(), None);
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    // The files are no items of their own: they go with their sticks.
    assert_eq!(
        app.backup_items(),
        vec![bstep::VAULT, bstep::ENVELOPE, bstep::STICKS]
    );
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(
        let_seed_files_out(&mut app),
        2,
        "Seed B's and Seed C's files, past the warning"
    );
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    let seed_b = stick_name("seed", fps[1], "txt");
    let seed_c = stick_name("seed", fps[2], "txt");
    {
        let f = app.flow().unwrap();
        let targets: Vec<Target> = f.slots.iter().map(|s| s.target).collect();
        assert_eq!(targets, vec![Target::Place(0), Target::PlaceFiles(1)]);
        assert_eq!(f.slots[0].files, vec![seed_b.clone()], "beside the vault");
        assert_eq!(f.slots[1].files, vec![seed_c.clone()]);
    }
    // One visit: the vault and Seed B's file, both on S1.
    write_stick(&mut shell, &mut app, "S1");
    {
        let f = app.flow().unwrap();
        let w = f.slots[0].written.as_ref().expect("S1 written");
        let wrote: Vec<&str> = w.files.iter().map(|(n, _)| n.as_str()).collect();
        assert!(wrote.contains(&seed_b.as_str()), "{wrote:?}");
        assert!(wrote.iter().any(|n| n.ends_with(".ofv")), "{wrote:?}");
        assert!(f.slots[1].written.is_none());
    }
    let on_s1 = names(&shell, "S1");
    assert!(on_s1.contains(&seed_b), "{on_s1:?}");
    assert!(!on_s1.contains(&seed_c), "{on_s1:?}");
    // Put back while the next stick is wanted: it is the one written.
    shell.plug(&mut app, "S1");
    assert_eq!(lit(&app), [2]);
    expect(&mut app, "This is Stick 2");
    shell.pull(&mut app, "S1");
    write_stick(&mut shell, &mut app, "S2");
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    assert!(names(&shell, "S2").contains(&seed_c));
}

/// A key made here and kept nowhere holds The plan; kept only as its
/// SeedQR picture on a stick, the checklist is made and the key stays
/// held until that stick is written (F31).
#[test]
fn a_seed_kept_nowhere_holds_the_plan() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(faraday_core::keygen::Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 5 + i / 3) % 2 == 0));
    }
    app.press(Action::KNext);
    app.press(Action::KLock);
    let fp = app.session.keys.last().unwrap().fingerprint().0;
    app.press(Action::KNext);
    app.press(Action::CBackup(0));
    untick_everywhere(&mut app, Row::Words);
    form_off_everywhere(&mut app, Form::Paper);
    assert!(app.backup_kept_nowhere().is_some(), "kept nowhere");
    app.press(Action::BChecklist);
    assert_eq!(
        app.backup.as_ref().unwrap().stage,
        BStage::Plan,
        "a seed kept nowhere holds The plan"
    );
    // Its SeedQR as a picture on a stick at Place 1: kept.
    board_edit(&mut app, |m| {
        plan::with_added(&m, At::Stick(0, 0), What::SeedQrFile(0))
    });
    assert_eq!(app.backup_kept_nowhere(), None);
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    assert_eq!(app.backup_items(), vec![bstep::ENVELOPE, bstep::STICKS]);
    app.press(Action::BNext(bstep::ENVELOPE));
    assert!(
        app.key_held(Fingerprint(fp)),
        "every item but the write is done, and the key waits for its stick"
    );
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(
        let_seed_files_out(&mut app),
        1,
        "the seed's file, past the warning"
    );
    let mut app = restart(&mut shell, &mut app);
    // No key is loaded after the lock: the flow holds it back.
    let held = app.backup_flows[0].wallet.as_ref().unwrap().held.clone();
    let name = faraday_core::wallet::fp_text(Fingerprint(fp));
    assert!(
        held.iter().any(|h| h.eq_ignore_ascii_case(&name)),
        "held across the lock: {held:?}"
    );
    assert!(app.backup_flows[0].waiting());
    write_stick(&mut shell, &mut app, "S1");
    assert_eq!(step(&app), Step::Done);
    assert!(
        names(&shell, "S1").contains(&stick_name("seedqr", fp, "png")),
        "{:?}",
        names(&shell, "S1")
    );
    assert!(!app.backup_flows[0].waiting(), "released once written");
}

fn seed(name: &str) -> Seed {
    Seed {
        name: name.to_string(),
        here: true,
        passphrase: false,
    }
}

fn two_of_three() -> Shape {
    Shape {
        m: 2,
        keys: 3,
        seeds: vec![seed("aaaa0001"), seed("aaaa0002"), seed("aaaa0003")],
        splits: true,
        vaults: 0,
    }
}

/// A plan saved before the sticks, its map as `spot` lines: each vault's
/// stick at a place a stick of its own, in vault order, the place's files
/// on a stick of files after them, the description's files in every form
/// its `form` answers had on; it makes the checklist it made before, and
/// the plan written out now reads back as it is.
#[test]
fn an_old_plan_text_loads_with_its_sticks_and_makes_the_same_checklist() {
    let shape = two_of_three();
    // The answers as a plan of then wrote them, its two forms on.
    let head = Answers::preset(&shape, Preset::Paper)
        .to_text()
        .replace("form 10\n", "form 11\n");
    assert!(head.contains("form 11\n") && !head.contains("spot "));
    // Paper and vaults, as the place cards kept them.
    let old = format!(
        "{head}spot place0 w0 sheet stick1 stick0\nspot place1 w1 sheet\nspot place2 w2 sheet stick0\nspot vault0 seed0 wallet\nspot vault1 seed1 wallet\n"
    );
    let a = Answers::from_text(&shape, &old).expect("the old plan loads");
    let m = plan::map(&shape, &a);
    assert_eq!(held(&m, At::Place(0)), vec![What::Words(0), What::Sheet]);
    assert_eq!(held(&m, At::Stick(0, 0)), vec![What::VaultStick(0)]);
    assert_eq!(held(&m, At::Stick(0, 1)), vec![What::VaultStick(1)]);
    assert_eq!(held(&m, At::Stick(2, 0)), vec![What::VaultStick(0)]);
    assert_eq!(plan::sticks_at(&m, 1), 0);
    assert_eq!(
        checklist(&shape, &a),
        vec![
            Item::Templates,
            Item::Copy(0),
            Item::Copy(1),
            Item::Copy(2),
            Item::Vault(0),
            Item::Vault(1),
            Item::Sheets,
            Item::Envelopes,
            Item::Sticks,
        ],
        "the checklist it made before"
    );
    let targets: Vec<Target> = BackupFlow::new_wallet(shape.clone(), a.clone(), "w")
        .slots
        .iter()
        .map(|s| s.target)
        .collect();
    assert_eq!(
        targets,
        vec![
            Target::PlaceVault(0, 0),
            Target::PlaceVault(0, 1),
            Target::Place(2),
            Target::Public
        ],
        "the sticks it wrote before, the sheets' stick last"
    );
    // A place's files, saved before the sticks: a stick of files after
    // the vault's, the description's files in both forms.
    let old = format!(
        "{head}spot place0 w0 file1 stick0 walletfile sharefile2\nspot place1 w1\nspot place2 w2\nspot vault0 seed0\n"
    );
    let a = Answers::from_text(&shape, &old).expect("the old plan loads");
    let m = plan::map(&shape, &a);
    assert_eq!(held(&m, At::Place(0)), vec![What::Words(0)]);
    assert_eq!(held(&m, At::Stick(0, 0)), vec![What::VaultStick(0)]);
    assert_eq!(
        held(&m, At::Stick(0, 1)),
        vec![
            What::SeedFile(1),
            What::WalletFile(form::QR),
            What::WalletFile(form::TEXT),
            What::ShareFile(2, form::QR),
            What::ShareFile(2, form::TEXT),
        ]
    );
    assert_eq!(plan::sticks_at(&m, 2), 0);
    let targets: Vec<Target> = BackupFlow::new_wallet(shape.clone(), a.clone(), "w")
        .slots
        .iter()
        .map(|s| s.target)
        .collect();
    assert_eq!(targets, vec![Target::Place(0), Target::PlaceFiles(0)]);
    // With the forms only the board makes (a SeedQR picture, a passphrase
    // file, the software's file) and written out now, sticks and all, it
    // reads back as it is.
    let mut a = a;
    let m = plan::with_added(&m, At::Stick(2, 0), What::SeedQrFile(0));
    let m = plan::with_added(&m, At::Stick(2, 0), What::PassphraseFile(0));
    let m = plan::with_added(&m, At::Stick(0, 0), What::SoftwareFile);
    a.set_map(m);
    let text = a.to_text();
    assert!(
        text.contains("spot stick0.1 file1 walletqr wallettxt shareqr2 sharetxt2\n"),
        "{text}"
    );
    assert!(
        text.contains("spot stick0.0 stick0 softwarefile\n"),
        "{text}"
    );
    assert!(text.contains("spot stick2.0 qrfile0 passfile0\n"), "{text}");
    assert_eq!(Answers::from_text(&shape, &text), Some(a.clone()));
    // A seed whose only copy outside paper is a file on a stick is kept.
    let m = plan::without(&plan::map(&shape, &a), At::Place(1), What::Words(1));
    a.set_map(m);
    assert_eq!(kept_nowhere(&shape, &a, &[0, 1, 2]), None);
    let m = plan::without(&plan::map(&shape, &a), At::Place(0), What::SeedFile(1));
    a.set_map(m);
    assert!(kept_nowhere(&shape, &a, &[0, 1, 2]).is_some());
}

/// A 2-of-3 made in Create with one vault set up on stick S1 before its
/// three keys (typed in), its backup open on Back up's plan with nothing
/// kept: no words, no paper. The keys' fingerprints.
fn three_with_a_vault(shell: &mut Shell) -> (Faraday, Vec<[u8; 4]>) {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CVaultN(1));
    app.press(Action::CNext(cstep::HOW_MANY));
    assert_eq!(step(&app), Step::Sticks);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    create_vault(shell, &mut app, 0, PHRASE);
    assert_eq!(app.screen, Screen::Create, "{:?}", app.vaults.unlock_error);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(0));
    untick_everywhere(&mut app, Row::Words);
    form_off_everywhere(&mut app, Form::Paper);
    (app, fps)
}

/// Vault 1 kept on two sticks at Place 1 (§19.12 item 7): the write step
/// writes two sticks, each its own write and its own pill, and each put
/// back afterwards is recognised as the one it is.
#[test]
fn two_sticks_of_vault_1_at_one_place_are_written_as_two_sticks_each_recognised() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let (mut app, _) = three_with_a_vault(&mut shell);
    keep_at(&mut app, 0, Row::Vault(0), &[0, 1, 2]);
    board_edit(&mut app, |m| plan::with_vault_stick(&m, 0, 0));
    let m = the_map(&app);
    assert_eq!(held(&m, At::Stick(0, 0)), vec![What::VaultStick(0)]);
    assert_eq!(held(&m, At::Stick(0, 1)), vec![What::VaultStick(0)]);
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(let_seed_files_out(&mut app), 0, "no seed's file");
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    let targets: Vec<Target> = app.flow().unwrap().slots.iter().map(|s| s.target).collect();
    assert_eq!(
        targets,
        vec![Target::PlaceVault(0, 0), Target::Another(0, Some(0), 1)],
        "a stick each"
    );
    // S1, read for Vault 1 when it was set up, takes the first; put back
    // while the second is wanted, it is lit as itself.
    write_stick(&mut shell, &mut app, "S1");
    shell.plug(&mut app, "S1");
    assert_eq!(lit(&app), [2]);
    expect(&mut app, "This is Stick 2");
    shell.pull(&mut app, "S1");
    // The second copy goes on S2 past the hold, a write of its own.
    write_stick(&mut shell, &mut app, "S2");
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let f = app.flow().unwrap();
    assert!(f.slots.iter().all(|s| s.written.is_some()));
    assert_ne!(
        f.slots[0].written.as_ref().unwrap().serial,
        f.slots[1].written.as_ref().unwrap().serial,
        "two sticks"
    );
    for d in ["S1", "S2"] {
        assert!(
            names(&shell, d).iter().any(|n| n.ends_with(".ofv")),
            "{d}: {:?}",
            names(&shell, d)
        );
    }
}

/// Two sticks of files at Place 1 (§19.4): Seed A's words on one, Seed
/// B's and Seed C's on the other. Each is its own write with its own
/// files, and each put back is recognised as the one it is.
#[test]
fn two_sticks_of_files_at_one_place_are_written_as_two_sticks_each_recognised() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(0));
    untick_everywhere(&mut app, Row::Words);
    form_off_everywhere(&mut app, Form::Paper);
    board_edit(&mut app, |m| {
        let m = plan::with_added(&m, At::Stick(0, 0), What::SeedFile(0));
        let m = plan::with_added(&m, At::Stick(0, 1), What::SeedFile(1));
        plan::with_added(&m, At::Stick(0, 1), What::SeedFile(2))
    });
    let m = the_map(&app);
    assert_eq!(held(&m, At::Stick(0, 0)), vec![What::SeedFile(0)]);
    assert_eq!(
        held(&m, At::Stick(0, 1)),
        vec![What::SeedFile(1), What::SeedFile(2)]
    );
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(let_seed_files_out(&mut app), 3);
    let mut app = restart(&mut shell, &mut app);
    let seed = |i: usize| stick_name("seed", fps[i], "txt");
    {
        let f = app.flow().unwrap();
        let targets: Vec<Target> = f.slots.iter().map(|s| s.target).collect();
        assert_eq!(
            targets,
            vec![Target::PlaceFiles(0), Target::Another(0, None, 1)]
        );
        assert_eq!(f.slots[0].files, vec![seed(0)]);
        assert_eq!(f.slots[1].files, vec![seed(1), seed(2)]);
    }
    write_stick(&mut shell, &mut app, "S1");
    // Put back while the second is wanted: it is the first, written, and
    // not taken for the second.
    shell.plug(&mut app, "S1");
    assert_eq!(lit(&app), [2]);
    expect(&mut app, "This is Stick 2");
    assert!(app.flow().unwrap().slots[1].written.is_none());
    shell.pull(&mut app, "S1");
    write_stick(&mut shell, &mut app, "S2");
    assert_eq!(step(&app), Step::Done);
    assert!(names(&shell, "S1").contains(&seed(0)));
    assert!(!names(&shell, "S1").contains(&seed(1)));
    assert!(names(&shell, "S2").contains(&seed(1)));
    assert!(names(&shell, "S2").contains(&seed(2)));
}

/// A plan that keeps a seed's words file on a stick shows the secret
/// sheet's warning at **Lock and write**, before anything is made or
/// written: until the person ticks the line and lets it out, no file is
/// made and nothing locks.
#[test]
fn a_seeds_words_file_on_a_stick_is_made_only_past_the_warning() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = three_here();
    keep_time(&mut app);
    untick_everywhere(&mut app, Row::Words);
    form_off_everywhere(&mut app, Form::Paper);
    keep_at(&mut app, 0, Row::File, &[0, 1, 2]);
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(app.sheet, Some(Sheet::SecretOut), "the warning first");
    let out = app.secret_out.as_ref().unwrap();
    assert_eq!(out.ack, faraday_core::secrets::Ack::Seed);
    assert!(out.name.starts_with("seed-") && out.name.ends_with(".txt"));
    let nothing_yet = |app: &Faraday, shell: &Shell| {
        assert!(app.outbox.iter().all(|i| !i.secret), "no seed file made");
        assert!(app.backup_flows.iter().all(|f| f.step != Step::Write));
        assert!(!app.restart_requested(), "no lock");
        assert!(shell.writes.is_empty(), "nothing written");
    };
    nothing_yet(&app, &shell);
    // Without the line ticked, nothing goes out.
    app.press(Action::SecretUnprotected);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    nothing_yet(&app, &shell);
    // Past it for Seed A, the next seed's warning; past all three, the
    // files are made and the lock comes.
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    assert_eq!(app.sheet, Some(Sheet::SecretOut), "Seed B's warning");
    nothing_yet(&app, &shell);
    assert_eq!(let_seed_files_out(&mut app), 2);
    assert_eq!(app.outbox.iter().filter(|i| i.secret).count(), 3);
    shell.pump(&mut app);
    assert!(app.restart_requested(), "locked");
}
