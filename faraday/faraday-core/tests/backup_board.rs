//! The backup board (`docs/BACKUP-FLOW.md` §19, F37) and its model:
//! sticks at a place (`At::Stick`), each with at most one vault and files
//! beside it; a vault one vault wherever its sticks are; the files made
//! and written with their stick, under their own names; a seed kept
//! nowhere holding the plan, and a seed kept only on a stick held until
//! that stick is written (F31); a plan saved before the sticks read as it
//! was. The board itself as a person uses it, by a press, a move past the
//! slop and a lift, or a tap and a tap: places added and removed, a chip
//! dragged or placed, its forms, vaults at several places, warnings,
//! cosigners, shares and the software place. Driven through Back up's
//! board and the flow.

mod common;

use common::*;
use faraday_core::backup_flow::{BackupFlow, Step, Target};
use faraday_core::backup_flow_steps::FlowAction as F;
use faraday_core::create::NewKind;
use faraday_core::plan::{
    self, Answers, At, Item, Preset, Seed, Shape, Spot, What, checklist, form, kept_nowhere,
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
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
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
    vault_at(&mut app, 0, 0, &[0]);
    vault_at(&mut app, 2, 0, &[0, 1]);
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
    vault_at(&mut app, 0, 0, &[0]);
    vault_at(&mut app, 0, 1, &[1]);
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
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    // Seed A in Vault 1 at Place 1; Seed C as a file at Place 2; nothing
    // on paper.
    no_words(&mut app);
    no_paper_description(&mut app);
    vault_at(&mut app, 0, 0, &[0]);
    files_at(&mut app, 1, &[2]);
    // Seed B's words file dropped on Vault 1's stick at Place 1.
    put(&mut app, Thing::Seed(1), Zone::Files(0, 0));
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
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    no_words(&mut app);
    no_paper_description(&mut app);
    assert!(app.backup_kept_nowhere().is_some(), "kept nowhere");
    app.press(Action::BChecklist);
    assert_eq!(
        app.backup.as_ref().unwrap().stage,
        BStage::Plan,
        "a seed kept nowhere holds The plan"
    );
    // Its SeedQR as a picture on a stick at Place 1: kept. Dropped on
    // + stick it is its words file; its forms make it the picture alone.
    put(&mut app, Thing::Seed(0), Zone::NewStick(0));
    form(&mut app, Zone::Files(0, 0), Thing::Seed(0), 1);
    form(&mut app, Zone::Files(0, 0), Thing::Seed(0), 0);
    assert_eq!(
        held(&the_map(&app), At::Stick(0, 0)),
        vec![What::SeedQrFile(0)]
    );
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
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    no_words(&mut app);
    no_paper_description(&mut app);
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
    vault_at(&mut app, 0, 0, &[0, 1, 2]);
    // Vault 1 dropped again on Place 1: a second stick of it.
    put(&mut app, Thing::Vault(0), Zone::Card(0));
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
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    no_words(&mut app);
    no_paper_description(&mut app);
    put(&mut app, Thing::Seed(0), Zone::NewStick(0));
    put(&mut app, Thing::Seed(1), Zone::NewStick(0));
    put(&mut app, Thing::Seed(2), Zone::Files(0, 1));
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
    no_words(&mut app);
    no_paper_description(&mut app);
    files_at(&mut app, 0, &[0, 1, 2]);
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

// ---- the board, as a person uses it (§19.1, §19.2, §19.3, §19.5) ----

use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use osk_shell_api::TouchPhase;

/// A 2-of-3 made in Create over three keys typed in here, its backup
/// opened from the wallet with no plan yet (the board on The standard
/// way), Customize backup pressed and the board emptied by its ×s: one
/// place keeping nothing.
fn three_on_an_empty_board() -> Faraday {
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
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    open_board(&mut app);
    app.press(Action::Board(BP::Customize));
    app.press(Action::Board(BP::RemovePlace(2)));
    app.press(Action::Board(BP::RemovePlace(1)));
    app.press(Action::Board(BP::Off(Zone::Paper(0), Thing::Seed(0))));
    app.press(Action::Board(BP::Off(Zone::Paper(0), Thing::Whole)));
    app.settle();
    app
}

/// The board's card opened, as a person opens it.
fn open_board(app: &mut Faraday) {
    if app.backup.as_ref().unwrap().q != Some(qstep::BOARD) {
        app.press(Action::BQ(qstep::BOARD));
    }
    app.settle();
}

fn touch(app: &mut Faraday, (x, y): (u16, u16), phase: TouchPhase) {
    app.event(Event::Touch { x, y, phase });
}

/// Where the frame on screen offers board press `p`, the page scrolled
/// from its top a step at a time until it does.
fn on_board(app: &mut Faraday, p: BP) -> (u16, u16) {
    on_screen(app, Action::Board(p))
}

/// Where the frame on screen offers `a`, the page scrolled from its top a
/// step at a time until it does.
fn on_screen(app: &mut Faraday, a: Action) -> (u16, u16) {
    find(app, a).unwrap_or_else(|| panic!("{a:?} is not on screen"))
}

/// Whether the page offers `a` anywhere down it.
fn offered_on_page(app: &mut Faraday, a: Action) -> bool {
    find(app, a).is_some()
}

/// [`on_screen`], or `None` where the page offers `a` nowhere.
fn find(app: &mut Faraday, a: Action) -> Option<(u16, u16)> {
    // A drop that let a place see the balance asks (§19.5): kept. The
    // board takes changes once Customize backup is pressed (§19.6).
    if app.sheet == Some(faraday_core::Sheet::BoardSees) {
        app.press(Action::Board(BP::SeesKeep));
    }
    if matches!(a, Action::Board(p) if !matches!(p, BP::Customize | BP::SeesKeep | BP::SeesUndo))
        && app.board_plan().is_some_and(|(_, plan)| !plan.custom)
    {
        app.press(Action::Board(BP::Customize));
    }
    let right = 1326;
    for x in [600, right] {
        app.event(Event::Scroll {
            x,
            y: 400,
            dy: -20_000,
        });
    }
    for _ in 0..30 {
        // A card that has just opened glides into view: what is pressed
        // is where it comes to rest.
        app.settle();
        let _ = app.frame();
        app.settle();
        let _ = app.frame();
        if let Some(at) = app.where_offered(a) {
            return Some(at);
        }
        app.event(Event::Scroll {
            x: 600,
            y: 400,
            dy: 150,
        });
        // The list column beside the cards scrolls on its own.
        let right = 1326;
        app.event(Event::Scroll {
            x: right,
            y: 400,
            dy: 150,
        });
    }
    None
}

/// [`expect`] for a line under the board: the page scrolled from its top
/// a step at a time until it is drawn (the checks stand above it).
/// [`expect_down`] for a line that may wrap: its start drawn.
fn expect_down_part(app: &mut Faraday, part: &str) {
    app.event(Event::Scroll {
        x: 600,
        y: 400,
        dy: -20_000,
    });
    for _ in 0..30 {
        app.settle();
        if app.drawn_texts().iter().any(|t| t.contains(part)) {
            return;
        }
        app.event(Event::Scroll {
            x: 600,
            y: 400,
            dy: 150,
        });
    }
    panic!("{part:?} is not drawn down the page");
}

fn expect_down(app: &mut Faraday, want: &str) {
    app.event(Event::Scroll {
        x: 600,
        y: 400,
        dy: -20_000,
    });
    for _ in 0..30 {
        app.settle();
        if app.drawn_texts().iter().any(|t| t == want) {
            return;
        }
        app.event(Event::Scroll {
            x: 600,
            y: 400,
            dy: 150,
        });
    }
    expect(app, want);
}

/// A tap on board press `p`: down and up where it is.
fn tap(app: &mut Faraday, p: BP) {
    let at = on_board(app, p);
    touch(app, at, TouchPhase::Down);
    touch(app, at, TouchPhase::Up);
    app.settle();
}

/// `from` dragged onto `to` as a finger drags it: down on it, moved past
/// the slop, moved over `to`, lifted there. Both are on screen at once.
fn drag(app: &mut Faraday, from: BP, to: BP) {
    let start = on_board(app, from);
    let end = app
        .where_offered(Action::Board(to))
        .unwrap_or_else(|| panic!("{to:?} is not on screen with {from:?}"));
    touch(app, start, TouchPhase::Down);
    touch(app, (start.0 + 12, start.1 + 4), TouchPhase::Move);
    let _ = app.frame();
    touch(app, end, TouchPhase::Move);
    let _ = app.frame();
    touch(app, end, TouchPhase::Up);
    app.settle();
}

fn list(t: Thing) -> BP {
    BP::Chip(Chip::List(t))
}

fn placed(z: Zone, t: Thing) -> BP {
    BP::Chip(Chip::Placed(z, t))
}

/// The board starts with one empty place and nothing kept; **+** adds a
/// place, and × removes it with what it kept.
#[test]
fn the_board_starts_with_one_empty_place_and_plus_and_x_add_and_remove_places() {
    let mut app = three_on_an_empty_board();
    assert_eq!(backup_places(&app), 1);
    let m = the_map(&app);
    assert!(
        m.iter()
            .filter(|s| !matches!(s.at, At::Away))
            .all(|s| s.holds.is_empty()),
        "nothing kept: {m:?}"
    );
    assert_eq!(plan::sticks_at(&m, 0), 0, "no stick yet");
    expect(&mut app, "Place 1");
    assert!(!has(&mut app, "Place 2"));
    // No × with one place.
    assert!(!offered(&mut app, Action::Board(BP::RemovePlace(0))));
    tap(&mut app, BP::AddPlace);
    assert_eq!(backup_places(&app), 2);
    expect(&mut app, "Place 2");
    // Place 2 keeps Seed A's words; its × takes them with it.
    tap(&mut app, list(Thing::Seed(0)));
    tap(&mut app, BP::Zone(Zone::Paper(1)));
    assert_eq!(held(&the_map(&app), At::Place(1)), vec![What::Words(0)]);
    tap(&mut app, BP::RemovePlace(1));
    assert_eq!(backup_places(&app), 1);
    assert!(
        the_map(&app)
            .iter()
            .all(|s| s.holds.iter().all(|(w, _)| *w != What::Words(0))),
        "what it kept went with it"
    );
    assert!(!has(&mut app, "Place 2"));
}

/// Seed A dragged onto Place 1's paper keeps its words there; its toggles
/// on the chip add the SeedQR, and the last form on stays on
/// (`docs/BACKUP-FLOW.md` §19.3); the map holds both.
#[test]
fn seed_a_dragged_onto_paper_keeps_its_words_and_its_forms_add_the_seedqr() {
    let mut app = three_on_an_empty_board();
    drag(&mut app, list(Thing::Seed(0)), BP::Zone(Zone::Paper(0)));
    assert_eq!(held(&the_map(&app), At::Place(0)), vec![What::Words(0)]);
    // The chip's toggles, with no popup: SeedQR turned on.
    let seedqr = BP::Form(Zone::Paper(0), Thing::Seed(0), 1);
    assert!(
        offered(&mut app, Action::Board(seedqr)),
        "its toggles are on the chip"
    );
    expect(&mut app, "Words");
    expect(&mut app, "SeedQR");
    tap(&mut app, seedqr);
    assert_eq!(
        held(&the_map(&app), At::Place(0)),
        vec![What::Words(0), What::SeedQr(0)]
    );
    // Words off keeps the SeedQR; the last form on stays on.
    tap(&mut app, BP::Form(Zone::Paper(0), Thing::Seed(0), 0));
    assert_eq!(held(&the_map(&app), At::Place(0)), vec![What::SeedQr(0)]);
    tap(&mut app, BP::Form(Zone::Paper(0), Thing::Seed(0), 1));
    assert_eq!(held(&the_map(&app), At::Place(0)), vec![What::SeedQr(0)]);
    tap(&mut app, BP::Form(Zone::Paper(0), Thing::Seed(0), 0));
    assert_eq!(
        held(&the_map(&app), At::Place(0)),
        vec![What::SeedQr(0), What::Words(0)]
    );
}

/// Tap to place makes the same map as a drag.
#[test]
fn tap_to_place_gives_the_same_map_as_a_drag() {
    let mut dragged = three_on_an_empty_board();
    drag(&mut dragged, list(Thing::Seed(0)), BP::Zone(Zone::Paper(0)));
    let mut tapped = three_on_an_empty_board();
    tap(&mut tapped, list(Thing::Seed(0)));
    // The zones that take it are edged; nothing is placed yet.
    assert_eq!(
        tapped.backup.as_ref().unwrap().board.held,
        Some(Chip::List(Thing::Seed(0)))
    );
    assert!(held(&the_map(&tapped), At::Place(0)).is_empty());
    tap(&mut tapped, BP::Zone(Zone::Paper(0)));
    assert_eq!(the_map(&tapped), the_map(&dragged));
    // A tap on a chip and then on it again lets it go.
    tap(&mut tapped, list(Thing::Seed(1)));
    tap(&mut tapped, list(Thing::Seed(1)));
    assert_eq!(tapped.backup.as_ref().unwrap().board.held, None);
}

/// Vault 1 dragged onto Place 1 and onto Place 3 makes a stick at each;
/// Seed A dropped into it at Place 1 is drawn in it at Place 3 too, and
/// the map holds it once, in the vault. A second vault dropped on Place 1
/// goes on a second stick there.
#[test]
fn a_vault_on_two_places_holds_seed_a_once_and_a_second_vault_takes_a_second_stick() {
    let mut app = three_on_an_empty_board();
    tap(&mut app, BP::AddPlace);
    tap(&mut app, BP::AddPlace);
    tap(&mut app, BP::AddVault);
    drag(&mut app, list(Thing::Vault(0)), BP::Zone(Zone::Card(0)));
    drag(&mut app, list(Thing::Vault(0)), BP::Zone(Zone::Card(2)));
    let m = the_map(&app);
    assert_eq!(held(&m, At::Stick(0, 0)), vec![What::VaultStick(0)]);
    assert_eq!(held(&m, At::Stick(2, 0)), vec![What::VaultStick(0)]);
    drag(
        &mut app,
        list(Thing::Seed(0)),
        BP::Zone(Zone::Vault(0, 0, 0)),
    );
    let m = the_map(&app);
    assert_eq!(held(&m, At::Vault(0)), vec![What::Seed(0)]);
    let at: Vec<At> = m
        .iter()
        .filter(|s| s.holds.iter().any(|(w, _)| *w == What::Seed(0)))
        .map(|s| s.at)
        .collect();
    assert_eq!(at, vec![At::Vault(0)], "held once");
    on_board(&mut app, placed(Zone::Vault(2, 0, 0), Thing::Seed(0)));
    // The second vault: a stick of its own at Place 1.
    tap(&mut app, BP::AddVault);
    drag(&mut app, list(Thing::Vault(1)), BP::Zone(Zone::Card(0)));
    let m = the_map(&app);
    assert_eq!(held(&m, At::Stick(0, 0)), vec![What::VaultStick(0)]);
    assert_eq!(held(&m, At::Stick(0, 1)), vec![What::VaultStick(1)]);
}

/// A key with a passphrase, made in Create on its own, its backup open on
/// the board with its words at both places.
fn passphrase_key_on_the_board() -> Faraday {
    use faraday_core::keygen::Way;
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 5 + i / 3) % 2 == 0));
    }
    app.press(Action::KNext);
    for field in [0, 1] {
        app.press(Action::KPassField(field));
        type_text(&mut app, "Ride the 7 bus");
    }
    app.press(Action::KLock);
    app.press(Action::KNext);
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    open_board(&mut app);
    // Over two places, as a plan saved before one place per key (§19.19).
    preset_two_places(&mut app, 1);
    app
}

/// A passphrase dropped on paper that keeps its seed's words is kept
/// there, with the warning line and the warning mark; Make the checklist
/// is still offered. Dropped into a vault that does not hold its seed, it
/// is kept there alone (§19.12 item 4).
#[test]
fn a_passphrase_beside_its_words_is_kept_with_a_warning_and_the_checklist_offered() {
    let mut app = passphrase_key_on_the_board();
    let words = held(&the_map(&app), At::Place(0));
    assert!(words.contains(&What::Words(0)), "{words:?}");
    // Its passphrase kept nowhere yet holds nothing back but a warning:
    // put it somewhere first, then beside the words.
    for p in 0..backup_places(&app) {
        for w in held(&the_map(&app), At::Place(p)) {
            if w == What::Passphrase(0) {
                tap(&mut app, BP::Off(Zone::Paper(p), Thing::Pass(0)));
            }
        }
    }
    drag(&mut app, list(Thing::Pass(0)), BP::Zone(Zone::Paper(0)));
    let m = the_map(&app);
    assert!(held(&m, At::Place(0)).contains(&What::Passphrase(0)));
    assert!(held(&m, At::Place(0)).contains(&What::Words(0)));
    let seed = backup_shape(&app).seeds[0].name.clone();
    expect_down(
        &mut app,
        &format!("Place 1 keeps {seed}'s words with its passphrase"),
    );
    let view = app
        .board_marks()
        .into_iter()
        .filter(|(z, _)| *z == Zone::Paper(0))
        .map(|(_, t)| t)
        .collect::<Vec<Thing>>();
    assert!(view.contains(&Thing::Pass(0)) && view.contains(&Thing::Seed(0)));
    app.press(Action::BQ(qstep::PLAN));
    assert!(
        offered_on_page(&mut app, Action::BChecklist),
        "still offered"
    );
    // Into a vault without its seed: the passphrase alone.
    open_board(&mut app);
    tap(&mut app, BP::AddVault);
    drag(&mut app, list(Thing::Vault(0)), BP::Zone(Zone::Card(1)));
    drag(
        &mut app,
        list(Thing::Pass(0)),
        BP::Zone(Zone::Vault(1, 0, 0)),
    );
    assert_eq!(
        held(&the_map(&app), At::Vault(0)),
        vec![What::PassphraseSealed(0)]
    );
}

/// A key made here and kept nowhere holds The plan, on the board as on
/// the cards; its words put back on paper let it go.
#[test]
fn a_seed_kept_nowhere_on_the_board_holds_the_plan() {
    let mut app = passphrase_key_on_the_board();
    for p in 0..backup_places(&app) {
        for w in held(&the_map(&app), At::Place(p)) {
            if let What::Words(_) = w {
                tap(&mut app, BP::Off(Zone::Paper(p), Thing::Seed(0)));
            }
        }
    }
    assert!(app.backup_kept_nowhere().is_some());
    let seed = backup_shape(&app).seeds[0].name.clone();
    expect_down(&mut app, &format!("Seed {seed} is kept nowhere"));
    app.press(Action::BQ(qstep::PLAN));
    assert!(
        !offered_on_page(&mut app, Action::BChecklist),
        "The plan is held"
    );
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Plan);
    open_board(&mut app);
    drag(&mut app, list(Thing::Seed(0)), BP::Zone(Zone::Paper(1)));
    app.press(Action::BQ(qstep::PLAN));
    assert!(offered_on_page(&mut app, Action::BChecklist));
}

/// A cosigner's seed is in the list, dim, "Not here", and does not move.
#[test]
fn a_cosigners_seed_is_dim_and_does_not_move() {
    let mut app = testkit::started();
    app.session = testkit::session();
    key_in(&mut app, 0);
    app.session
        .add_wallet(
            "Savings",
            &testkit::savings(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    open_board(&mut app);
    app.press(Action::Board(BP::Customize));
    let shape = backup_shape(&app);
    let away = (0..shape.seeds.len())
        .find(|&i| !shape.seeds[i].here)
        .expect("a cosigner");
    expect(&mut app, "Not here");
    assert!(
        !offered(&mut app, Action::Board(list(Thing::Seed(away)))),
        "it takes no press"
    );
    // A finger on its name, dragged to Place 1's paper: nothing moves.
    let _ = app.frame();
    let name = shape.seeds[away].name.clone();
    let (_, (x, y, bw, bh)) = app
        .drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| *t == name)
        .expect("its chip is drawn");
    let start = ((x + bw / 2) as u16, (y + bh / 2) as u16);
    let end = on_board(&mut app, BP::Zone(Zone::Paper(0)));
    let before = the_map(&app);
    touch(&mut app, start, TouchPhase::Down);
    touch(&mut app, (start.0 + 20, start.1), TouchPhase::Move);
    touch(&mut app, end, TouchPhase::Move);
    touch(&mut app, end, TouchPhase::Up);
    assert_eq!(the_map(&app), before);
    assert_eq!(app.backup.as_ref().unwrap().board.held, None);
    // Tapping it and then a zone places nothing either.
    app.press(Action::Board(list(Thing::Seed(away))));
    app.press(Action::Board(BP::Zone(Zone::Paper(0))));
    assert_eq!(the_map(&app), before);
}

/// Partial descriptions by one button (`docs/BACKUP-FLOW.md` §19.19):
/// no share is offered by itself; the button hands one to every place
/// and the sheet says the whole is still kept at every place; Take it off
/// every place leaves the shares alone, and the plan survives any one
/// place lost; a place taken off or added has them handed out again;
/// Remove partial descriptions takes every share off, and the warning
/// says the description cannot be put back together.
#[test]
fn partial_descriptions_are_added_by_one_button_and_follow_the_places() {
    let mut app = fresh_three();
    app.press(Action::Board(BP::Customize));
    let _ = app.frame();
    assert!(!app.offers(Action::Board(list(Thing::Share(0)))));
    assert!(!app.offers(Action::BOmit(0)), "no stepper");
    expect(&mut app, "Add partial descriptions to this backup");
    app.press(Action::Board(BP::Partials));
    assert_eq!(app.sheet, Some(Sheet::BoardPartials));
    let t = app.drawn_texts().join(" ");
    assert!(
        t.contains("Each place now holds a part of the description."),
        "{t}"
    );
    assert!(
        t.contains("The whole description is still kept at Place 1, Place 2 and Place 3"),
        "{t}"
    );
    assert!(app.offers(Action::Board(BP::PartialsWholeOff)), "{t}");
    app.press(Action::Board(BP::PartialsWholeOff));
    assert_eq!(app.sheet, None);
    let shape = backup_shape(&app);
    let a = app.backup.as_ref().unwrap().answers.clone();
    assert_eq!(a.omit, 1);
    for p in 0..3 {
        assert!(!a.sheet_at(&shape, p), "Place {}: the whole is off", p + 1);
        assert_eq!(a.shares_at(&shape, p).len(), 1, "Place {}", p + 1);
    }
    assert_eq!(plan::check(&shape, &a).lost, plan::Lost::Yes);
    // A place taken off and one added: every place has its share.
    app.press(Action::Board(BP::RemovePlace(2)));
    app.press(Action::Board(BP::AddPlace));
    let a = app.backup.as_ref().unwrap().answers.clone();
    assert_eq!(a.places, 3);
    for p in 0..3 {
        assert_eq!(a.shares_at(&shape, p).len(), 1, "Place {}", p + 1);
    }
    expect_down(&mut app, "Place 3 holds no seed");
    // Remove partial descriptions: no share, and the description cannot
    // be put back together.
    expect(&mut app, "Remove partial descriptions");
    app.press(Action::Board(BP::Partials));
    let a = app.backup.as_ref().unwrap().answers.clone();
    assert!((0..3).all(|p| a.shares_at(&shape, p).is_empty()));
    expect_down_part(&mut app, "The description cannot be put back together");
}

/// A single key's one place: "Any one place lost" reads "No: a single
/// place", muted, not as a warning (`docs/BACKUP-FLOW.md` §19.19).
#[test]
fn a_single_keys_one_place_reads_no_a_single_place() {
    let mut app = testkit::started();
    let fp = key_in(&mut app, 1);
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    let _ = app.frame();
    expect(&mut app, "No: a single place");
    let t = app.drawn_texts().join(" ");
    assert!(
        !t.contains("Separate places are strongly recommended"),
        "{t}"
    );
    assert!(!t.contains("is kept only at"), "{t}");
}

/// The software place: its card, the description scanned in, and its
/// stick with the description's files; losing every numbered place, the
/// checks count the description as kept.
#[test]
fn the_software_place_scans_the_description_in_and_keeps_it_through_any_loss() {
    let mut app = three_on_an_empty_board();
    tap(&mut app, BP::AddPlace);
    tap(&mut app, BP::AddPlace);
    for p in 0..3 {
        drag(&mut app, list(Thing::Seed(p)), BP::Zone(Zone::Paper(p)));
    }
    let shape = backup_shape(&app);
    let lost = |app: &Faraday| plan::check(&shape, &app.backup.as_ref().unwrap().answers).lost;
    assert_eq!(lost(&app), plan::Lost::No, "no description kept");
    tap(&mut app, list(Thing::Software));
    expect(&mut app, "Scanned in");
    assert_eq!(held(&the_map(&app), At::Software), vec![What::Wallet]);
    assert!(!offered(&mut app, Action::Board(list(Thing::Software))));
    drag(&mut app, list(Thing::Whole), BP::Zone(Zone::SoftStick));
    assert_eq!(held(&the_map(&app), At::Files), vec![What::Wallet]);
    assert_eq!(lost(&app), plan::Lost::Yes);
    // The software is a card of the plan too: its × takes it off.
    tap(&mut app, BP::RemoveSoftware);
    assert!(held(&the_map(&app), At::Software).is_empty());
    assert!(offered(&mut app, Action::Board(list(Thing::Software))));
}

/// A placed chip moved to another zone takes its forms with it, a words
/// file for words and a SeedQR picture for a SeedQR; dragged back onto
/// the list, or by its ×, it is taken off.
#[test]
fn a_placed_chip_moves_with_its_forms_and_back_to_the_list_is_taken_off() {
    let mut app = three_on_an_empty_board();
    drag(&mut app, list(Thing::Seed(0)), BP::Zone(Zone::Paper(0)));
    app.press(Action::Board(BP::Chip(Chip::Placed(
        Zone::Paper(0),
        Thing::Seed(0),
    ))));
    app.press(Action::Board(BP::Form(Zone::Paper(0), Thing::Seed(0), 1)));
    app.press(Action::Board(BP::LetGo));
    drag(
        &mut app,
        placed(Zone::Paper(0), Thing::Seed(0)),
        BP::Zone(Zone::NewStick(0)),
    );
    let m = the_map(&app);
    assert!(held(&m, At::Place(0)).is_empty());
    assert_eq!(
        held(&m, At::Stick(0, 0)),
        vec![What::SeedFile(0), What::SeedQrFile(0)]
    );
    drag(
        &mut app,
        placed(Zone::Files(0, 0), Thing::Seed(0)),
        BP::Zone(Zone::List),
    );
    assert!(
        the_map(&app).iter().all(|s| s
            .holds
            .iter()
            .all(|(w, _)| !matches!(w, What::SeedFile(0) | What::SeedQrFile(0)))),
        "taken off"
    );
    assert_eq!(
        plan::sticks_at(&the_map(&app), 0),
        0,
        "the stick went with it"
    );
}

/// A vault's × on the list takes it off with its sticks; one that holds
/// something says what first, and goes on **Take off**.
#[test]
fn a_vault_that_holds_something_says_what_before_it_is_taken_off() {
    let mut app = three_on_an_empty_board();
    tap(&mut app, BP::AddVault);
    drag(&mut app, list(Thing::Vault(0)), BP::Zone(Zone::Card(0)));
    drag(
        &mut app,
        list(Thing::Seed(0)),
        BP::Zone(Zone::Vault(0, 0, 0)),
    );
    tap(&mut app, BP::RemoveVault(0));
    assert!(
        the_map(&app).iter().any(|s| s.at == At::Vault(0)),
        "asked first"
    );
    let seed = backup_shape(&app).seeds[0].name.clone();
    expect(&mut app, &format!("Vault 1 holds {seed}"));
    tap(&mut app, BP::RemoveVaultSure(0));
    let m = the_map(&app);
    assert!(m.iter().all(|s| s.at != At::Vault(0)));
    assert_eq!(plan::sticks_at(&m, 0), 0);
}

/// A 2-of-3 over three keys typed in here, made in Create with Vault 1
/// set up before the keys: vault "Main" on S1, unlocked, holding another
/// wallet's description (the spending kit, returned). Its backup open on
/// the board.
fn main_vault_board(shell: &mut Shell) -> (Faraday, testkit::Kit) {
    use faraday_vault::records::{field, kind};
    sticks(shell, 1);
    // Vault "Main" on S1, holding another wallet's description.
    let other = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .expect("the spending kit");
    let main = {
        let cost = faraday_vault::Cost {
            memory_kib: faraday_vault::MIN_MEMORY_KIB,
            passes: 1,
            lanes: 1,
        };
        let contents = vec![faraday_vault::Contents {
            records: vec![
                faraday_vault::Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Main"),
                faraday_vault::Record::new(kind::WALLET)
                    .with(field::WALLET, other.descriptor.as_bytes()),
            ],
        }];
        faraday_vault::create(
            faraday_vault::SLOT_SIZES[0],
            cost,
            &[PHRASE.as_bytes()],
            &contents,
            &[0x41; 32],
        )
        .unwrap()
    };
    shell.disk_mut("S1").files = vec![("main.ofv".into(), main)];
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CVaultN(1));
    app.press(Action::CNext(cstep::HOW_MANY));
    // S1's vault is Vault 1: picked, the place confirmed.
    shell.plug(&mut app, "S1");
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .unwrap()
        .files
        .iter()
        .position(|(f, _)| f.name == "main.ofv")
        .expect("the vault is read");
    shell.flow(&mut app, F::PickFile(0, k));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    assert_eq!(step(&app), Step::Vaults);
    unlock(&mut app, PHRASE);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    open_board(&mut app);
    (app, other)
}

/// A vault that already holds something (made before this backup) shows
/// what it held in its zone, dimmed, taking no press (§19.12 item 7).
#[test]
fn an_existing_vaults_contents_are_drawn_dimmed_and_take_no_press() {
    let mut shell = Shell::new();
    let (mut app, other) = main_vault_board(&mut shell);
    if vault_stick(&app, 0, 0).is_none() {
        tap(&mut app, list(Thing::Vault(0)));
        // On **+ stick**: the card's middle is a chip of its paper.
        tap(&mut app, BP::Zone(Zone::NewStick(0)));
    }
    let sum = faraday_core::wallet::read_wallet(&other.descriptor)
        .unwrap()
        .checksum()
        .to_string();
    expect(&mut app, &format!("#{sum}"));
    // It is no chip of the plan's: nothing to press, nothing to drag.
    assert!(!offered(
        &mut app,
        Action::Board(placed(Zone::List, Thing::Whole))
    ));
    assert!(
        the_map(&app)
            .iter()
            .all(|s| s.at != At::Vault(0) || s.holds.iter().all(|(w, _)| *w != What::Wallet))
    );
}

// ---- F2b: the guided flow's board, the narrow column, the boxes ----

/// The plan the guided flow's plan card edits: its board opened there,
/// before the lock, as a person opens it (`docs/BACKUP-FLOW.md` §19.12
/// item 1).
fn open_flow_board(shell: &mut Shell, app: &mut Faraday) {
    let i = app.backup_flows.len() - 1;
    shell.flow(app, F::Show(i));
    shell.flow(app, F::ChangePlan);
    if app.flow().unwrap().live.plan_q != Some(qstep::BOARD) {
        shell.flow(app, F::Plan(faraday_core::PlanPress::Q(qstep::BOARD)));
    }
    assert!(matches!(
        app.board_of(),
        Some(faraday_core::board::BoardOf::Flow(_))
    ));
}

/// The flow's own plan's map.
fn flow_map(app: &Faraday) -> Vec<Spot> {
    let wf = app.flow().unwrap().wallet.as_ref().unwrap();
    plan::map(&wf.shape, &wf.answers)
}

/// The same presses on a board: Vault 1 holding Seeds A and B at Place
/// 1, Seed C's words and SeedQR on its paper, the description sealed in
/// Vault 1, a place more with Share 1 on its paper and Seed C's words
/// file on a stick there.
fn the_same_plan(app: &mut Faraday) {
    vault_at(app, 0, 0, &[0, 1]);
    put(app, Thing::Seed(2), Zone::Paper(0));
    form(app, Zone::Paper(0), Thing::Seed(2), 1);
    let s = vault_stick(app, 0, 0).unwrap();
    put(app, Thing::Whole, Zone::Vault(0, s, 0));
    board(app, BP::AddPlace);
    put(app, Thing::Share(0), Zone::Paper(1));
    put(app, Thing::Seed(2), Zone::NewStick(1));
}

/// The board in the guided flow's plan card is Back up's board on the
/// flow's own plan: the same presses make the same map.
#[test]
fn a_plan_made_on_the_flows_board_is_the_map_back_up_makes() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let (mut backup, _) = three_with_a_vault(&mut shell);
    open_board(&mut backup);
    the_same_plan(&mut backup);
    let mut shell2 = Shell::new();
    sticks(&mut shell2, 3);
    let (mut flow, _) = three_with_a_vault(&mut shell2);
    open_flow_board(&mut shell2, &mut flow);
    the_same_plan(&mut flow);
    assert_eq!(flow_map(&flow), the_map(&backup));
    // The flow draws its board: its places and its list.
    expect(&mut flow, "Place 2");
    expect(&mut flow, "Seeds");
}

/// After the lock, on the guided flow's board, paper and a stick's files
/// take nothing and say §17.8's line; a vault still changes, and the
/// change is a round of the vault (`docs/BACKUP-FLOW.md` §19.12 item 8).
#[test]
fn after_the_lock_the_flows_board_keeps_paper_and_files_and_changes_vaults() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let (mut app, _) = three_with_a_vault(&mut shell);
    vault_at(&mut app, 0, 0, &[0, 1, 2]);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    shell.flow(&mut app, F::ChangePlan);
    shell.flow(&mut app, F::Plan(faraday_core::PlanPress::Q(qstep::BOARD)));
    assert!(matches!(
        app.board_of(),
        Some(faraday_core::board::BoardOf::Flow(_))
    ));
    let line = "Files and paper stay as made after the lock; finish the writes, then Back up again to change them";
    let before = flow_map(&app);
    // A press on paper says why nothing there changes.
    assert_ne!(app.toast_text(), Some(line));
    board(&mut app, BP::Zone(Zone::Paper(0)));
    assert_eq!(app.toast_text(), Some(line));
    // Seed A's words on paper, the description as a file: refused.
    put(&mut app, Thing::Seed(0), Zone::Paper(0));
    put(&mut app, Thing::Whole, Zone::NewStick(0));
    assert_eq!(app.toast_text(), Some(line));
    assert_eq!(flow_map(&app), before, "paper and files as made");
    // The description sealed in Vault 1: taken, a round of the vault.
    let s = vault_stick(&app, 0, 0).unwrap();
    put(&mut app, Thing::Whole, Zone::Vault(0, s, 0));
    shell.pump(&mut app);
    assert!(held(&flow_map(&app), At::Vault(0)).contains(&What::Wallet));
    assert!(app.flow().unwrap().adding, "{:?}", app.toast_text());
}

/// The display of a small panel: the `compact` layout, the narrow column.
fn narrow(app: &mut Faraday) {
    app.event(Event::Display(osk_shell_api::DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: osk_shell_api::SecureHardware::None,
        boot: osk_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    app.settle();
}

/// A narrow window (`docs/BACKUP-FLOW.md` §19.12 item 3, §19.20 item 4):
/// the place cards stand in one column; **Add** opens the list as a
/// sheet, each item with its ⋮; a tap on a chip there opens its Add to…
/// as its ⋮ does, and a box chosen places it: the map a drag makes on
/// `wide`.
#[test]
fn a_narrow_window_places_from_the_add_sheets_menu_as_a_drag_does_on_wide() {
    let mut wide = three_on_an_empty_board();
    drag(&mut wide, list(Thing::Seed(0)), BP::Zone(Zone::Paper(0)));
    let mut app = three_on_an_empty_board();
    narrow(&mut app);
    assert!(app.is_compact());
    // Two places, one under the other.
    tap(&mut app, BP::AddPlace);
    let a = on_board(&mut app, BP::RemovePlace(0));
    let b = on_board(&mut app, BP::RemovePlace(1));
    assert_eq!(a.0, b.0, "one column");
    tap(&mut app, BP::RemovePlace(1));
    // The list is not on the page: Add opens it.
    assert!(!offered(&mut app, Action::Board(list(Thing::Seed(0)))));
    tap(&mut app, BP::OpenList);
    assert_eq!(app.sheet, Some(Sheet::BoardList));
    for t in [Thing::Seed(0), Thing::Seed(1), Thing::Whole] {
        assert!(
            offered(&mut app, Action::Board(BP::Menu(Chip::List(t)))),
            "{t:?}'s ⋮"
        );
    }
    tap(&mut app, list(Thing::Seed(0)));
    assert_eq!(app.sheet, Some(Sheet::BoardMenu), "its Add to…");
    expect(&mut app, "Add to…");
    assert_eq!(
        app.backup.as_ref().unwrap().board.held,
        None,
        "nothing held"
    );
    tap(&mut app, BP::MenuTo(Zone::Paper(0)));
    assert_eq!(app.sheet, None, "the menu closes on the board");
    assert_eq!(the_map(&app), the_map(&wide));
    // The Add sheet's ⋮ places an item the same way.
    tap(&mut app, BP::OpenList);
    tap(&mut app, BP::Menu(Chip::List(Thing::Seed(1))));
    tap(&mut app, BP::MenuTo(Zone::Paper(0)));
    assert!(held(&the_map(&app), At::Place(0)).contains(&What::Words(1)));
    // A placed chip's forms open as on `wide`.
    tap(&mut app, placed(Zone::Paper(0), Thing::Seed(0)));
    assert!(offered_on_page(
        &mut app,
        Action::Board(BP::Form(Zone::Paper(0), Thing::Seed(0), 1))
    ));
}

/// What is sealed in a vault and what is only a file on its stick are
/// told apart as a person reads them (`docs/BACKUP-FLOW.md` §19.14,
/// §19.15, §19.17): under the stick's head "Stick 1", the files box
/// headed "Files · not sealed" with Seed B's words file and SeedQR
/// picture in it, and under it, apart from it, the vault box headed
/// "Vault 1 · sealed · made in the checklist" (a vault not made yet)
/// with Seed A inside; Paper is beside them.
#[test]
fn a_seed_in_a_vault_and_a_seed_file_on_its_stick_are_drawn_in_their_own_boxes() {
    let mut app = three_on_an_empty_board();
    put(&mut app, Thing::Vault(0), Zone::Card(0));
    let s = vault_stick(&app, 0, 0).unwrap();
    put(&mut app, Thing::Seed(0), Zone::Vault(0, s, 0));
    put(&mut app, Thing::Seed(1), Zone::Files(0, s));
    form(&mut app, Zone::Files(0, s), Thing::Seed(1), 1);
    let m = the_map(&app);
    assert_eq!(held(&m, At::Vault(0)), vec![What::Seed(0)]);
    assert_eq!(
        held(&m, At::Stick(0, s)),
        vec![What::VaultStick(0), What::SeedFile(1), What::SeedQrFile(1)]
    );
    let inside = |inner: (u16, u16, u16, u16), outer: (u16, u16, u16, u16)| {
        inner.0 >= outer.0
            && inner.1 >= outer.1
            && inner.0 + inner.2 <= outer.0 + outer.2
            && inner.1 + inner.3 <= outer.1 + outer.3
    };
    let _ = on_board(&mut app, BP::Zone(Zone::Vault(0, s, 0)));
    let boxed = |app: &Faraday, p: BP| app.hit_box(Action::Board(p)).unwrap();
    let files = boxed(&app, BP::Zone(Zone::Files(0, s)));
    let vault = boxed(&app, BP::Zone(Zone::Vault(0, s, 0)));
    let paper = boxed(&app, BP::Zone(Zone::Paper(0)));
    let sealed = boxed(&app, placed(Zone::Vault(0, s, 0), Thing::Seed(0)));
    let file = boxed(&app, placed(Zone::Files(0, s), Thing::Seed(1)));
    assert!(
        files.1 + files.3 <= vault.1,
        "the files box is above the vault box, apart from it"
    );
    assert!(inside(sealed, vault), "Seed A is inside the vault box");
    assert!(
        inside(file, files) && !inside(file, vault),
        "Seed B's file is in the files box, outside the vault"
    );
    assert!(
        paper.0 >= files.0 + files.2 && paper.0 >= vault.0 + vault.2,
        "Paper is beside the stick box, outside it"
    );
    // Each box has its head row: "Stick 1" above the files box, its
    // header at the top of it, "Vault 1" at the top of the vault box;
    // Seed B's chip reads its two file forms.
    let texts = app.drawn_text_boxes();
    let at = |t: &str| {
        texts
            .iter()
            .find(|(s, _)| s == t)
            .map(|(_, b)| *b)
            .unwrap_or_else(|| panic!("{t:?} not drawn"))
    };
    let stick_head = at("Stick 1");
    let files_head = at("Files · not sealed");
    // A vault added on the board, not made yet (§19.21 item 6).
    let vault_head = texts
        .iter()
        .find(|(s, _)| s == "Vault 1 · sealed · made in the checklist" || s == "Vault 1 · new")
        .map(|(_, b)| *b)
        .expect("Vault 1's band");
    assert!(
        stick_head.1 < i32::from(files.1),
        "Stick 1 heads the stick box"
    );
    assert!(
        files_head.1 >= i32::from(files.1) && files_head.1 < i32::from(file.1),
        "Files · not sealed heads the files box"
    );
    assert!(
        vault_head.1 >= i32::from(vault.1) && vault_head.1 < i32::from(sealed.1),
        "Vault 1's band heads the vault box"
    );
    // Its forms are toggles on the chip (§19.3).
    for form in ["Words file", "SeedQR file"] {
        assert!(texts.iter().any(|(s, _)| s == form), "no {form} toggle");
    }
}

/// On `wide` the board reads a place to a row (`docs/BACKUP-FLOW.md`
/// §19.15 item 1): under Place 1's header, its two stick boxes, **+
/// stick** and Paper stand side by side, left to right, on one row; Place
/// 2 is under Place 1, its own row under its own header.
#[test]
fn on_wide_a_places_sticks_and_paper_stand_on_one_row_and_the_places_one_under_another() {
    let mut app = three_on_an_empty_board();
    tap(&mut app, BP::AddPlace);
    put(&mut app, Thing::Vault(0), Zone::Card(0));
    put(&mut app, Thing::Whole, Zone::NewStick(0));
    put(&mut app, Thing::Seed(0), Zone::Paper(0));
    put(&mut app, Thing::Seed(1), Zone::Paper(1));
    let v = vault_stick(&app, 0, 0).unwrap();
    let f = files_stick(&app, 0).unwrap();
    let _ = on_board(&mut app, BP::Zone(Zone::Paper(0)));
    let boxed = |app: &Faraday, z: Zone| {
        app.hit_box(Action::Board(BP::Zone(z)))
            .unwrap_or_else(|| panic!("{z:?} is not drawn"))
    };
    let first = boxed(&app, Zone::Files(0, v.min(f)));
    let second = boxed(&app, Zone::Files(0, v.max(f)));
    let add = boxed(&app, Zone::NewStick(0));
    let paper = boxed(&app, Zone::Paper(0));
    let paper_2 = boxed(&app, Zone::Paper(1));
    // Side by side, left to right, each starting where the one before it
    // ends, on one row: the sticks' files boxes at one height, under
    // their heads, **+ stick** and Paper from the row's top.
    let row = [first, second, add, paper];
    for pair in row.windows(2) {
        assert!(pair[0].0 + pair[0].2 <= pair[1].0, "left to right: {row:?}");
    }
    assert_eq!(first.1, second.1, "one row: {row:?}");
    assert_eq!(add.1, paper.1, "one row: {row:?}");
    assert!(
        add.1 < first.1 && first.1 < add.1 + add.3,
        "one row: {row:?}"
    );
    let texts = app.drawn_text_boxes();
    let at = |t: &str| {
        texts
            .iter()
            .find(|(s, _)| s == t)
            .map(|(_, b)| *b)
            .unwrap_or_else(|| panic!("{t:?} not drawn"))
    };
    let (place_1, place_2) = (at("Place 1"), at("Place 2"));
    assert!(
        place_1.1 + place_1.3 <= i32::from(first.1),
        "the row is under Place 1's header"
    );
    assert!(
        at("Stick 1").0 < at("Stick 2").0 && at("Stick 2").0 < at("Paper").0,
        "Stick 1, Stick 2 and Paper read left to right"
    );
    // Place 2 under Place 1's row, at the same left edge.
    assert!(
        i32::from(paper.1 + paper.3) <= place_2.1,
        "Place 2 is under Place 1"
    );
    assert_eq!(place_1.0, place_2.0, "one under another");
    assert!(place_2.1 + place_2.3 <= i32::from(paper_2.1));
}

// ---- F2d: + stick, a vault at no place, the files box, a name pressed ----

/// How strongly a text drawn on the frame on screen stands out: the most
/// any pixel of its ink differs from the corner of its box, the fill
/// under it.
fn ink_light(app: &mut Faraday, text: &str) -> u32 {
    let boxes = app.drawn_text_boxes();
    let (_, (x, y, w, h)) = boxes
        .into_iter()
        .find(|(t, _)| t == text)
        .unwrap_or_else(|| panic!("{text:?} not drawn"));
    let f = app.frame();
    let width = f.width as i32;
    let at = |px: i32, py: i32| -> i32 {
        let i = ((py * width + px) * 4) as usize;
        f.rgba
            .get(i..i + 3)
            .map_or(0, |p| i32::from(p[0]) + i32::from(p[1]) + i32::from(p[2]))
    };
    let under = at(x.max(0), y.max(0));
    let mut most = 0;
    for py in y.max(0)..(y + h) {
        for px in x.max(0)..(x + w) {
            most = most.max((at(px, py) - under).unsigned_abs());
        }
    }
    most
}

/// A press on **+ stick** with nothing held adds an empty stick there,
/// "Stick 1" with its files box and its ×, and a further **+ stick**;
/// The plan and the checklist do not change. A file dropped on it is kept
/// on a stick of the plan there; its × takes an empty stick away; a
/// place removed takes its empty sticks with it (`docs/BACKUP-FLOW.md`
/// §19.16 item 1, §19.17 item 1).
#[test]
fn a_press_on_plus_stick_adds_an_empty_stick_that_the_plan_does_not_see() {
    let mut app = three_on_an_empty_board();
    let shape = backup_shape(&app);
    let before = app.backup.as_ref().unwrap().answers.clone();
    tap(&mut app, BP::Zone(Zone::NewStick(0)));
    expect(&mut app, "Stick 1");
    expect(&mut app, "Files · not sealed");
    assert!(offered_on_page(
        &mut app,
        Action::Board(BP::Zone(Zone::Empty(0, 0)))
    ));
    assert!(offered_on_page(&mut app, Action::Board(BP::RemoveEmpty(0))));
    assert!(offered_on_page(
        &mut app,
        Action::Board(BP::Zone(Zone::NewStick(0)))
    ));
    let after = app.backup.as_ref().unwrap().answers.clone();
    assert_eq!(
        the_map(&app),
        plan::map(&shape, &before),
        "The plan unchanged"
    );
    assert_eq!(checklist(&shape, &after), checklist(&shape, &before));
    // Another: two empty sticks in a row.
    tap(&mut app, BP::Zone(Zone::NewStick(0)));
    expect(&mut app, "Stick 2");
    // Seed A dropped on the first is a file on Place 1's stick of the plan;
    // one empty stick is left after it.
    drag(&mut app, list(Thing::Seed(0)), BP::Zone(Zone::Empty(0, 0)));
    assert_eq!(
        held(&the_map(&app), At::Stick(0, 0)),
        vec![What::SeedFile(0)]
    );
    assert!(offered_on_page(
        &mut app,
        Action::Board(BP::Zone(Zone::Empty(0, 0)))
    ));
    assert!(!offered_on_page(
        &mut app,
        Action::Board(BP::Zone(Zone::Empty(0, 1)))
    ));
    expect(&mut app, "Stick 2");
    // Its × takes it away.
    tap(&mut app, BP::RemoveEmpty(0));
    assert!(!offered_on_page(
        &mut app,
        Action::Board(BP::Zone(Zone::Empty(0, 0)))
    ));
    assert_eq!(
        held(&the_map(&app), At::Stick(0, 0)),
        vec![What::SeedFile(0)]
    );
    // Place 2's empty stick goes with Place 2.
    tap(&mut app, BP::AddPlace);
    tap(&mut app, BP::Zone(Zone::NewStick(1)));
    assert!(offered_on_page(
        &mut app,
        Action::Board(BP::Zone(Zone::Empty(1, 0)))
    ));
    tap(&mut app, BP::RemovePlace(1));
    tap(&mut app, BP::AddPlace);
    assert!(!offered_on_page(
        &mut app,
        Action::Board(BP::Zone(Zone::Empty(1, 0)))
    ));
}

/// With Places 2 and 3 removed, the vaults whose sticks they kept are out
/// of The plan and the checklist, still on the list, drawn dim, with what
/// they hold; a seed kept only in one of them is kept nowhere; dragged
/// back onto a place, a vault returns with what it held
/// (`docs/BACKUP-FLOW.md` §19.16 item 3).
#[test]
fn a_vault_kept_at_no_place_is_out_of_the_plan_and_comes_back_with_what_it_held() {
    let mut app = three_on_an_empty_board();
    tap(&mut app, BP::AddPlace);
    tap(&mut app, BP::AddPlace);
    tap(&mut app, BP::AddVault);
    tap(&mut app, BP::AddVault);
    tap(&mut app, BP::AddVault);
    put(&mut app, Thing::Vault(0), Zone::Card(0));
    put(&mut app, Thing::Vault(1), Zone::Card(1));
    put(&mut app, Thing::Vault(2), Zone::Card(2));
    put(&mut app, Thing::Seed(0), Zone::Paper(0));
    put(&mut app, Thing::Seed(2), Zone::Paper(0));
    put(&mut app, Thing::Seed(0), Zone::Vault(0, 0, 0));
    put(&mut app, Thing::Seed(1), Zone::Vault(1, 0, 1));
    put(&mut app, Thing::Seed(2), Zone::Vault(2, 0, 2));
    tap(&mut app, BP::RemovePlace(2));
    tap(&mut app, BP::RemovePlace(1));
    let shape = backup_shape(&app);
    let a = app.backup.as_ref().unwrap().answers.clone();
    let m = the_map(&app);
    assert!(m.iter().any(|s| s.at == At::Vault(0)));
    for v in [1, 2] {
        assert!(
            m.iter().all(|s| s.at != At::Vault(v)),
            "Vault {} out of The plan",
            v + 1
        );
        assert!(!checklist(&shape, &a).contains(&Item::Vault(v)));
        assert!(offered_on_page(
            &mut app,
            Action::Board(list(Thing::Vault(v)))
        ));
    }
    // What they hold is kept with them.
    let stored = plan::stored_map(&shape, &a);
    assert_eq!(held(&stored, At::Vault(1)), vec![What::Seed(1)]);
    // Drawn dim beside Vault 1, which a place keeps.
    let (kept, away) = (
        ink_light(&mut app, "Vault 1"),
        ink_light(&mut app, "Vault 2"),
    );
    assert!(away < kept, "Vault 2 is drawn dim: {away} against {kept}");
    // Seed B was kept only in Vault 2.
    let line = kept_nowhere(&shape, &a, &[1]).expect("Seed B is kept nowhere");
    assert_eq!(
        line,
        format!("Seed {} is kept nowhere", shape.seeds[1].name)
    );
    expect_down(&mut app, &line);
    // Vault 2 dragged onto Place 1 is back with Seed B in it.
    drag(&mut app, list(Thing::Vault(1)), BP::Zone(Zone::Card(0)));
    let m = the_map(&app);
    assert_eq!(held(&m, At::Vault(1)), vec![What::Seed(1)]);
    let a = app.backup.as_ref().unwrap().answers.clone();
    assert!(kept_nowhere(&shape, &a, &[1]).is_none());
}

/// A vault set up before the keys whose stick no place keeps reads
/// "Vault 1's stick needs a place" under the board, and Make the
/// checklist is still offered; its × takes the warning away and leaves
/// its chip on the list, and dragged onto a place it is in The plan again
/// (`docs/BACKUP-FLOW.md` §19.16 item 3).
#[test]
fn a_vault_that_exists_at_no_place_is_a_warning_only() {
    let mut shell = Shell::new();
    let (mut app, _) = main_vault_board(&mut shell);
    for i in 0..3 {
        put(&mut app, Thing::Seed(i), Zone::Paper(0));
    }
    // Vault 1 in this plan, at Place 1; then its stick taken off.
    if vault_stick(&app, 0, 0).is_none() {
        put(&mut app, Thing::Vault(0), Zone::Card(0));
    }
    while let Some(s) = vault_stick(&app, 0, 0) {
        tap(&mut app, BP::RemoveStick(0, s));
    }
    let needs = "Vault 1's stick needs a place";
    expect_down(&mut app, needs);
    assert!(the_map(&app).iter().all(|s| s.at != At::Vault(0)));
    app.press(Action::BQ(qstep::PLAN));
    assert!(
        offered_on_page(&mut app, Action::BChecklist),
        "Make the checklist is offered"
    );
    open_board(&mut app);
    tap(&mut app, BP::RemoveVault(0));
    if offered_on_page(&mut app, Action::Board(BP::RemoveVaultSure(0))) {
        tap(&mut app, BP::RemoveVaultSure(0));
    }
    assert!(!has(&mut app, needs));
    assert!(offered_on_page(
        &mut app,
        Action::Board(list(Thing::Vault(0)))
    ));
    drag(&mut app, list(Thing::Vault(0)), BP::Zone(Zone::Card(0)));
    assert!(the_map(&app).iter().any(|s| s.at == At::Vault(0)));
    assert!(vault_stick(&app, 0, 0).is_some());
}

/// A drop on the files box of a stick that carries a vault keeps the
/// thing on the stick as a file, not in the vault; a stick with no vault
/// takes a drop on its files box the same; a new stick has its files box,
/// empty (`docs/BACKUP-FLOW.md` §19.17).
#[test]
fn a_drop_on_a_sticks_files_box_keeps_a_file_beside_its_vault() {
    let mut app = three_on_an_empty_board();
    tap(&mut app, BP::AddVault);
    drag(&mut app, list(Thing::Vault(0)), BP::Zone(Zone::Card(0)));
    let s = vault_stick(&app, 0, 0).unwrap();
    drag(&mut app, list(Thing::Seed(0)), BP::Zone(Zone::Files(0, s)));
    let m = the_map(&app);
    assert_eq!(
        held(&m, At::Stick(0, s)),
        vec![What::VaultStick(0), What::SeedFile(0)]
    );
    assert!(held(&m, At::Vault(0)).is_empty(), "nothing sealed");
    // A stick of files, no vault on it.
    tap(&mut app, BP::AddPlace);
    put(&mut app, Thing::Whole, Zone::NewStick(1));
    let f = files_stick(&app, 1).unwrap();
    drag(&mut app, list(Thing::Seed(1)), BP::Zone(Zone::Files(1, f)));
    assert!(held(&the_map(&app), At::Stick(1, f)).contains(&What::SeedFile(1)));
    // A new stick: its files box is there, empty.
    tap(&mut app, BP::Zone(Zone::NewStick(1)));
    let _ = on_board(&mut app, BP::Zone(Zone::Empty(1, 0)));
    let headers = app
        .drawn_texts()
        .iter()
        .filter(|t| *t == "Files · not sealed")
        .count();
    assert_eq!(headers, 3, "a files box on every stick");
}

/// A press on a place's name selects the whole of it: one key typed gives
/// a name of that one character (`docs/BACKUP-FLOW.md` §19.18).
#[test]
fn a_press_on_a_places_name_selects_it_and_the_first_key_replaces_it() {
    let mut shell = Shell::new();
    let (mut app, _) = main_vault_board(&mut shell);
    let name = on_screen(&mut app, Action::BName(0));
    touch(&mut app, name, TouchPhase::Down);
    touch(&mut app, name, TouchPhase::Up);
    app.settle();
    assert!(app.selected_all(), "the whole name selected");
    app.event(Event::Key(Key::Char('K')));
    assert_eq!(app.place_name(0), "K");
    app.event(Event::Key(Key::Char('i')));
    assert_eq!(app.place_name(0), "Ki");
    // Pressed again later, the name typed is selected and replaced.
    app.event(Event::Key(Key::Enter));
    app.press(Action::BName(0));
    app.event(Event::Key(Key::Backspace));
    app.event(Event::Key(Key::Char('L')));
    assert_eq!(app.place_name(0), "L");
    // An arrow lets the selection go: typed at the caret.
    app.event(Event::Key(Key::Enter));
    app.press(Action::BName(0));
    app.event(Event::Key(Key::End));
    app.event(Event::Key(Key::Char('2')));
    assert_eq!(app.place_name(0), "L2");
    app.event(Event::Key(Key::Home));
    app.event(Event::Key(Key::Char('>')));
    assert_eq!(app.place_name(0), ">L2");
}

// ---- The standard way, Customize backup, the sheet over a drop ----

/// A 2-of-3 made in Create over three keys typed in here, its backup
/// opened from the wallet with no plan yet.
fn fresh_three() -> Faraday {
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
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    let _ = app.frame();
    app
}

/// What place `p`'s paper holds, in order.
fn paper(app: &Faraday, p: usize) -> Vec<What> {
    held(&the_map(app), At::Place(p))
}

/// A fresh Back up on a 2-of-3 opens on the board, The standard way
/// placed (`docs/BACKUP-FLOW.md` §19.6, §19.21): three places, each its
/// seed's words and the whole description, no SeedQR, no vault, no stick;
/// no Start from, and the read-only board's two buttons.
#[test]
fn a_fresh_two_of_three_opens_on_the_board_with_the_standard_way_placed() {
    let mut app = fresh_three();
    let b = app.backup.as_ref().unwrap();
    assert_eq!(b.q, Some(qstep::BOARD), "the board is open");
    assert!(!b.answers.custom, "read-only");
    assert_eq!(backup_places(&app), 3);
    let m = the_map(&app);
    for p in 0..3 {
        assert_eq!(
            paper(&app, p),
            vec![What::Words(p), What::Sheet],
            "Place {}",
            p + 1
        );
        assert_eq!(plan::sticks_at(&m, p), 0, "no stick");
    }
    assert!(!m.iter().any(|s| matches!(s.at, At::Vault(_))), "no vault");
    let t = app.drawn_texts();
    assert!(!t.iter().any(|s| s == "Start from"), "{t:?}");
    let soft = t
        .iter()
        .position(|s| s == "Add watch-only software")
        .expect("Add watch-only software");
    assert_eq!(t[soft + 1], "Customize backup", "{t:?}");
}

/// A single key's standard way is one place: its words and the whole
/// description, no SeedQR (§19.21 item 12).
#[test]
fn a_single_keys_standard_way_is_one_place() {
    let mut app = testkit::started();
    let fp = key_in(&mut app, 1);
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    assert_eq!(app.backup.as_ref().unwrap().q, Some(qstep::BOARD));
    assert_eq!(backup_places(&app), 1);
    assert_eq!(paper(&app, 0), vec![What::Words(0), What::Sheet]);
}

/// Until Customize backup the board is read-only: the list's column is the
/// button, a press on a place's × or a chip's × changes nothing, and a
/// drag moves nothing. Customize backup shows the list, and a drag places
/// a chip.
#[test]
fn the_board_is_read_only_until_customize_backup() {
    let mut app = fresh_three();
    let before = the_map(&app);
    assert!(app.offers(Action::Board(BP::Customize)), "Customize backup");
    expect(&mut app, "Customize backup");
    for t in [Thing::Seed(0), Thing::Whole] {
        assert!(!app.offers(Action::Board(list(t))), "no list: {t:?}");
    }
    assert!(!app.offers(Action::Board(BP::RemovePlace(0))), "no ×");
    assert!(!app.offers(Action::Board(BP::AddPlace)), "no + place");
    assert!(!app.offers(Action::Board(BP::Off(Zone::Paper(0), Thing::Whole))));
    // Pressed anyway, nothing changes.
    app.press(Action::Board(BP::RemovePlace(0)));
    app.press(Action::Board(BP::Off(Zone::Paper(0), Thing::Whole)));
    app.press(Action::Board(BP::Form(Zone::Paper(0), Thing::Seed(0), 0)));
    assert_eq!(the_map(&app), before);
    // The read-only chip reads its forms as text.
    expect_part(&mut app, "· Words");
    // A drag of a placed chip onto another place moves nothing.
    let chip = Action::Board(placed(Zone::Paper(0), Thing::Seed(0)));
    let _ = app.frame();
    let (x, y, w, h) = app.hit_box(chip).expect("the chip is drawn");
    let start = (x + w / 2, y + h / 2);
    let (px, py, pw, ph) = app
        .hit_box(Action::Board(BP::Zone(Zone::Paper(1))))
        .expect("Place 2's paper");
    let end = (px + pw / 2, py + ph - 4);
    touch(&mut app, start, TouchPhase::Down);
    touch(&mut app, (start.0 + 12, start.1 + 4), TouchPhase::Move);
    touch(&mut app, end, TouchPhase::Move);
    touch(&mut app, end, TouchPhase::Up);
    app.settle();
    assert_eq!(the_map(&app), before, "the drag moved nothing");
    // Customize backup: the list, and a drag places a chip.
    app.press(Action::Board(BP::Customize));
    let _ = app.frame();
    assert!(app.offers(Action::Board(list(Thing::Seed(0)))), "the list");
    drag(&mut app, list(Thing::Seed(1)), BP::Zone(Zone::Paper(0)));
    assert!(
        paper(&app, 0).contains(&What::Words(1)),
        "{:?}",
        paper(&app, 0)
    );
    assert!(app.backup.as_ref().unwrap().answers.custom);
}

/// **Add watch-only software** on the read-only board adds the software
/// place with the description Scanned in, the board staying read-only;
/// it then reads **Take off watch-only software** and takes it off again
/// (`docs/BACKUP-FLOW.md` §19.21 item 3). The plan saved with it reads
/// back as it was.
#[test]
fn another_preset_replaces_the_standard_way_in_both_states() {
    let mut app = fresh_three();
    let _ = app.frame();
    assert!(app.offers(Action::Board(BP::Software)));
    app.press(Action::Board(BP::Software));
    let b = app.backup.as_ref().unwrap();
    assert!(!b.answers.custom, "still read-only");
    assert_eq!(held(&the_map(&app), At::Software), vec![What::Wallet]);
    let shape = backup_shape(&app);
    let text = b.answers.to_text();
    assert_eq!(Answers::from_text(&shape, &text).as_ref(), Some(&b.answers));
    let _ = app.frame();
    expect(&mut app, "Take off watch-only software");
    assert!(
        !app.drawn_texts()
            .iter()
            .any(|t| t == "Add watch-only software")
    );
    for p in 0..3 {
        assert_eq!(paper(&app, p), vec![What::Words(p), What::Sheet]);
    }
    app.press(Action::Board(BP::Software));
    assert!(held(&the_map(&app), At::Software).is_empty(), "taken off");
    assert!(!app.backup.as_ref().unwrap().answers.custom);
    let _ = app.frame();
    expect(&mut app, "Add watch-only software");
}

/// The checklist after The standard way writes those copies: the blank
/// templates, each seed's words and SeedQR by hand, and the description's
/// sheets.
#[test]
fn the_standard_ways_checklist_writes_its_copies() {
    let mut app = fresh_three();
    app.press(Action::BQ(qstep::PLAN));
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    let items = app.backup_items();
    for i in 0..3 {
        assert!(
            items.contains(&bstep::of(Item::Copy(i))),
            "seed {i}: {items:?}"
        );
    }
    assert!(items.contains(&bstep::of(Item::Sheets)), "{items:?}");
    assert!(items.contains(&bstep::of(Item::Templates)), "{items:?}");
    assert!(!items.contains(&bstep::of(Item::Vault(0))), "no vault");
}

/// In Customize backup a seed chip's toggles: the SeedQR is added by its
/// toggle (§19.21 item 12), Words off then keeps the SeedQR, and the last
/// form on cannot be turned off.
#[test]
fn a_seed_chips_toggles_keep_the_last_form_on() {
    let mut app = fresh_three();
    app.press(Action::Board(BP::Customize));
    let _ = app.frame();
    let words = BP::Form(Zone::Paper(0), Thing::Seed(0), 0);
    let seedqr = BP::Form(Zone::Paper(0), Thing::Seed(0), 1);
    assert!(app.offers(Action::Board(words)), "a toggle on the chip");
    app.press(Action::Board(words));
    assert_eq!(paper(&app, 0), vec![What::Words(0), What::Sheet], "stays");
    app.press(Action::Board(seedqr));
    assert_eq!(
        sorted(paper(&app, 0)),
        sorted(vec![What::Words(0), What::SeedQr(0), What::Sheet])
    );
    app.press(Action::Board(words));
    assert_eq!(
        sorted(paper(&app, 0)),
        sorted(vec![What::SeedQr(0), What::Sheet])
    );
    app.press(Action::Board(seedqr));
    assert_eq!(
        sorted(paper(&app, 0)),
        sorted(vec![What::SeedQr(0), What::Sheet]),
        "stays"
    );
}

/// A 2-of-3 customised with the whole description off Place 3: Share 1
/// leaves off key 3, whose seed Place 3 holds, so dropping it there lets
/// Place 3 see the balance and the sheet says so (§19.5).
fn share_sheet_board() -> Faraday {
    let mut app = fresh_three();
    app.press(Action::Board(BP::Customize));
    app.press(Action::Board(BP::Off(Zone::Paper(2), Thing::Whole)));
    app.press(Action::Board(list(Thing::Share(0))));
    app.press(Action::Board(BP::Zone(Zone::Paper(2))));
    app
}

#[test]
fn dropping_a_share_beside_the_seed_of_its_missing_key_asks_and_take_it_off_undoes_it() {
    let mut app = share_sheet_board();
    assert_eq!(app.sheet, Some(Sheet::BoardSees));
    assert!(paper(&app, 2).contains(&What::Share(0)));
    let fp = backup_shape(&app).seeds[2].name.clone();
    let t = app.drawn_texts().join(" ");
    assert!(t.contains("Place 3 can see the wallet's balance"), "{t}");
    assert!(
        t.contains(&format!(
            "Share 1 and seed {fp} together make the whole description."
        )),
        "{t}"
    );
    app.press(Action::Board(BP::SeesUndo));
    assert_eq!(app.sheet, None);
    assert!(!paper(&app, 2).contains(&What::Share(0)), "taken off");
}

#[test]
fn keep_it_keeps_the_drop() {
    let mut app = share_sheet_board();
    assert_eq!(app.sheet, Some(Sheet::BoardSees));
    app.press(Action::Board(BP::SeesKeep));
    assert_eq!(app.sheet, None);
    assert!(paper(&app, 2).contains(&What::Share(0)), "kept");
}

/// A share dropped where the place still cannot see the balance asks
/// nothing.
#[test]
fn a_share_on_a_place_that_still_cannot_see_asks_nothing() {
    let mut app = fresh_three();
    app.press(Action::Board(BP::Customize));
    app.press(Action::Board(BP::Off(Zone::Paper(0), Thing::Whole)));
    // Share 1 carries keys 1 and 2, Place 1 holds seed 1: key 3 missing.
    app.press(Action::Board(list(Thing::Share(0))));
    app.press(Action::Board(BP::Zone(Zone::Paper(0))));
    assert!(paper(&app, 0).contains(&What::Share(0)));
    assert_eq!(app.sheet, None);
}

// ---- §19.20: the ⋮ menu ----

/// The ⋮ menu of the chip `c`.
fn dots(c: Chip) -> BP {
    BP::Menu(c)
}

/// [`three_on_an_empty_board`] with a second place, Vault 1 on a stick at
/// Place 1, and the watch-only software's place.
fn menu_board() -> Faraday {
    let mut app = three_on_an_empty_board();
    app.press(Action::Board(BP::AddPlace));
    app.press(Action::Board(BP::AddVault));
    app.press(Action::Board(list(Thing::Vault(0))));
    app.press(Action::Board(BP::Zone(Zone::Card(0))));
    app.press(Action::Board(list(Thing::Software)));
    app.settle();
    assert!(vault_stick(&app, 0, 0).is_some(), "Vault 1 at Place 1");
    app
}

/// A seed's ⋮ in the list opens Add to…: every box that takes the seed,
/// under each place's name in order (Place 1's paper, its stick's files,
/// the vault on it, a new stick there; then Place 2's), and none that
/// does not (the software place's Scanned in). A box chosen places the
/// seed as a drop there does, and the menu closes
/// (`docs/BACKUP-FLOW.md` §19.20 items 2 and 6).
#[test]
fn add_to_on_a_seed_lists_the_boxes_that_take_it_by_place_and_places_it_as_a_drop() {
    let mut dropped = menu_board();
    drag(&mut dropped, list(Thing::Seed(0)), BP::Zone(Zone::Paper(1)));
    let mut app = menu_board();
    let s = vault_stick(&app, 0, 0).unwrap();
    tap(&mut app, dots(Chip::List(Thing::Seed(0))));
    assert_eq!(app.sheet, Some(Sheet::BoardMenu));
    assert_eq!(
        app.backup.as_ref().unwrap().board.held,
        None,
        "not selected"
    );
    expect(&mut app, "Add to…");
    let boxes = [
        Zone::Paper(0),
        Zone::Files(0, s),
        Zone::Vault(0, s, 0),
        Zone::NewStick(0),
        Zone::Paper(1),
        Zone::NewStick(1),
    ];
    let _ = app.frame();
    let ys: Vec<u16> = boxes
        .iter()
        .map(|z| {
            app.hit_box(Action::Board(BP::MenuTo(*z)))
                .unwrap_or_else(|| panic!("{z:?} is offered"))
                .1
        })
        .collect();
    assert!(ys.windows(2).all(|w| w[0] < w[1]), "in order: {ys:?}");
    assert!(
        !app.offers(Action::Board(BP::MenuTo(Zone::Scanned))),
        "Scanned in does not take a seed"
    );
    // Each place's name over its boxes.
    let heads = app.drawn_text_boxes();
    let between = |name: &str, lo: u16, hi: u16| {
        heads
            .iter()
            .any(|(t, (_, y, _, _))| t == name && *y > i32::from(lo) && *y < i32::from(hi))
    };
    assert!(between("Place 2", ys[3], ys[4]), "Place 2 over its boxes");
    assert!(
        between("Stick 1 · Vault 1", ys[1], ys[3]),
        "the vault named by its stick"
    );
    tap(&mut app, BP::MenuTo(Zone::Paper(1)));
    assert_eq!(app.sheet, None, "the menu closes");
    assert_eq!(the_map(&app), the_map(&dropped), "as the drop placed it");
}

/// A placed chip's ⋮: **Move to…** moves it with its forms, as a drag
/// does; **Copy to…** keeps it in both boxes, as a second drop from the
/// list does; **Take off** takes it off, as its × does (§19.20 items 3
/// and 6).
#[test]
fn a_placed_chips_menu_moves_copies_and_takes_off() {
    let start = |app: &mut Faraday| {
        app.press(Action::Board(list(Thing::Seed(0))));
        app.press(Action::Board(BP::Zone(Zone::Paper(0))));
        app.press(Action::Board(BP::Form(Zone::Paper(0), Thing::Seed(0), 1)));
        app.settle();
    };
    let chip = Chip::Placed(Zone::Paper(0), Thing::Seed(0));
    // Move to….
    let mut dragged = menu_board();
    start(&mut dragged);
    drag(&mut dragged, BP::Chip(chip), BP::Zone(Zone::Paper(1)));
    let mut app = menu_board();
    start(&mut app);
    tap(&mut app, dots(chip));
    expect(&mut app, "Move to…");
    expect(&mut app, "Copy to…");
    expect(&mut app, "Take off");
    tap(&mut app, BP::MenuVerb(faraday_core::board::Verb::Move));
    assert!(
        !offered(&mut app, Action::Board(BP::MenuTo(Zone::Paper(0)))),
        "not the box it is in"
    );
    tap(&mut app, BP::MenuTo(Zone::Paper(1)));
    assert_eq!(the_map(&app), the_map(&dragged));
    assert_eq!(
        held(&the_map(&app), At::Place(1)),
        vec![What::Words(0), What::SeedQr(0)],
        "with its forms"
    );
    assert!(held(&the_map(&app), At::Place(0)).is_empty());
    // Copy to….
    let mut app = menu_board();
    start(&mut app);
    tap(&mut app, dots(chip));
    tap(&mut app, BP::MenuVerb(faraday_core::board::Verb::Copy));
    expect(&mut app, "Copy to…");
    tap(&mut app, BP::MenuTo(Zone::Paper(1)));
    assert_eq!(
        held(&the_map(&app), At::Place(0)),
        vec![What::Words(0), What::SeedQr(0)],
        "still kept where it was"
    );
    assert_eq!(held(&the_map(&app), At::Place(1)), vec![What::Words(0)]);
    // Take off.
    let mut app = menu_board();
    start(&mut app);
    tap(&mut app, dots(chip));
    tap(&mut app, BP::Off(Zone::Paper(0), Thing::Seed(0)));
    assert_eq!(app.sheet, None);
    assert!(held(&the_map(&app), At::Place(0)).is_empty());
}

/// A press on a chip's ⋮ opens its menu and neither selects the chip
/// nor starts its drag, even when the finger moves (§19.20 item 1).
#[test]
fn a_press_on_the_dots_neither_selects_nor_drags() {
    let mut app = menu_board();
    let before = the_map(&app);
    let at = on_board(&mut app, dots(Chip::List(Thing::Seed(0))));
    touch(&mut app, at, TouchPhase::Down);
    touch(&mut app, (at.0 - 200, at.1 + 40), TouchPhase::Move);
    let _ = app.frame();
    let b = &app.backup.as_ref().unwrap().board;
    assert_eq!(b.held, None, "not held");
    assert_eq!(b.drag, None, "not dragged");
    touch(&mut app, (at.0 - 200, at.1 + 40), TouchPhase::Up);
    app.settle();
    assert_eq!(app.backup.as_ref().unwrap().board.held, None);
    assert_eq!(the_map(&app), before, "nothing placed");
}

/// The menu by the keys (§19.20 item 5): Tab reaches a chip's ⋮, Enter
/// opens its menu, the arrows move and Enter chooses; Escape goes back
/// from Move to… to the first level, then closes the menu with the
/// keyboard on the ⋮ again. A press outside the menu closes it, nothing
/// changed.
#[test]
fn the_menu_by_the_keys_and_escape_closes_it_back_to_its_dots() {
    let mut app = menu_board();
    app.press(Action::Board(list(Thing::Seed(0))));
    app.press(Action::Board(BP::Zone(Zone::Paper(0))));
    app.settle();
    let chip = Chip::Placed(Zone::Paper(0), Thing::Seed(0));
    let focus = |app: &Faraday| app.backup.as_ref().unwrap().board.focus;
    let _ = app.frame();
    for _ in 0..40 {
        if focus(&app) == Some(chip) {
            break;
        }
        app.event(Event::Key(Key::Tab));
    }
    assert_eq!(focus(&app), Some(chip), "Tab reaches the chip's ⋮");
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.sheet, Some(Sheet::BoardMenu));
    expect(&mut app, "Move to…");
    // Down to Copy to… and up again, to Move to…; Enter opens it.
    app.event(Event::Key(Key::Down));
    app.event(Event::Key(Key::Up));
    app.event(Event::Key(Key::Enter));
    expect(&mut app, "Paper");
    assert!(offered(&mut app, Action::Board(BP::MenuTo(Zone::Paper(1)))));
    // Escape: back to the first level, then closed, on the ⋮.
    app.event(Event::Key(Key::Escape));
    assert_eq!(app.sheet, Some(Sheet::BoardMenu));
    expect(&mut app, "Copy to…");
    app.event(Event::Key(Key::Escape));
    assert_eq!(app.sheet, None, "closed");
    assert_eq!(focus(&app), Some(chip), "the keyboard on its ⋮");
    // Enter opens it again (on the frame a shell draws after the key);
    // Down, Down to Take off; Enter.
    let _ = app.frame();
    app.event(Event::Key(Key::Enter));
    app.event(Event::Key(Key::Down));
    app.event(Event::Key(Key::Down));
    app.event(Event::Key(Key::Enter));
    assert!(held(&the_map(&app), At::Place(0)).is_empty(), "taken off");
    // A press outside closes it, nothing changed.
    let before = the_map(&app);
    tap(&mut app, dots(Chip::List(Thing::Seed(1))));
    assert_eq!(app.sheet, Some(Sheet::BoardMenu));
    let _ = app.frame();
    touch(&mut app, (4, 760), TouchPhase::Down);
    touch(&mut app, (4, 760), TouchPhase::Up);
    app.settle();
    assert_eq!(app.sheet, None, "closed by a press outside");
    assert_eq!(the_map(&app), before);
}

/// The read-only board (before Customize backup) shows no ⋮; once
/// customised every chip has one (§19.20 items 1 and 6).
#[test]
fn the_read_only_board_shows_no_dots() {
    let mut app = fresh_three();
    let chip = Chip::Placed(Zone::Paper(0), Thing::Seed(0));
    let _ = app.frame();
    for p in 0..3 {
        for c in [
            Chip::Placed(Zone::Paper(p), Thing::Seed(p)),
            Chip::Placed(Zone::Paper(p), Thing::Whole),
        ] {
            assert!(
                !offered(&mut app, Action::Board(BP::Menu(c))),
                "no ⋮ while read-only: {c:?}"
            );
        }
    }
    app.press(Action::Board(BP::Menu(chip)));
    assert_eq!(app.sheet, None, "nothing opens");
    app.press(Action::Board(BP::Customize));
    assert!(offered(&mut app, Action::Board(BP::Menu(chip))));
    assert!(offered(
        &mut app,
        Action::Board(BP::Menu(Chip::List(Thing::Seed(0))))
    ));
}

/// The text a vault box's header band reads, by the vault's title.
fn vault_band(app: &mut Faraday, title: &str) -> String {
    let _ = app.frame();
    app.drawn_texts()
        .into_iter()
        .find(|t| {
            t.starts_with(&format!("{title} ·")) && (t.contains("sealed") || t.ends_with(" · new"))
        })
        .unwrap_or_else(|| panic!("no band for {title}: {:?}", app.drawn_texts()))
}

/// A vault added on the board with **+ new vault** is made in the
/// checklist, and its box says so until it is (`docs/BACKUP-FLOW.md`
/// §19.21 items 4 and 6).
#[test]
fn a_vault_added_with_new_vault_reads_made_in_the_checklist() {
    let mut app = fresh_three();
    app.press(Action::Board(BP::Customize));
    let _ = app.frame();
    expect(&mut app, "+ new vault");
    app.press(Action::Board(BP::AddVault));
    app.press(Action::Board(BP::Menu(Chip::List(Thing::Vault(0)))));
    app.press(Action::Board(BP::MenuTo(Zone::NewStick(0))));
    assert!(
        vault_stick(&app, 0, 0).is_some(),
        "Vault 1's stick at Place 1"
    );
    let band = vault_band(&mut app, "Vault 1");
    assert!(
        band == "Vault 1 · sealed · made in the checklist" || band == "Vault 1 · new",
        "{band}"
    );
}

/// A vault set up before the keys is made already: its box reads
/// "· sealed" (`docs/BACKUP-FLOW.md` §19.21 item 6).
#[test]
fn a_vault_set_up_before_the_keys_reads_sealed() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CVaultN(1));
    app.press(Action::CNext(cstep::HOW_MANY));
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    vault_at(&mut app, 0, 0, &[0]);
    let band = vault_band(&mut app, "Vault 1");
    assert!(band.ends_with(" · sealed"), "{band}");
}

/// A plan saved from Paper and vault, before the presets were withdrawn,
/// reopens as it was saved and customisable; one from The standard way
/// keeps its read-only state (`docs/BACKUP-FLOW.md` §19.21 item 7).
#[test]
fn a_plan_saved_from_paper_and_vault_reopens_unchanged_and_customisable() {
    let shape = two_of_three();
    let a = Answers::preset(&shape, Preset::PaperVault);
    assert!(!a.to_text().contains("preset "), "no longer written");
    let saved = format!("{}preset 2\n", a.to_text());
    let back = Answers::from_text(&shape, &saved).expect("read, not refused");
    assert!(back.custom, "customisable");
    assert_eq!(plan::map(&shape, &back), plan::map(&shape, &a), "as saved");
    let standard = Answers::preset(&shape, Preset::Standard);
    let saved = format!("{}preset 0\n", standard.to_text());
    let back = Answers::from_text(&shape, &saved).expect("read");
    assert!(!back.custom, "The standard way stays read-only");
}

/// Change the plan, from the checklist, opens the board as it was left;
/// Customize backup, the words off and the SeedQR on, then Make the
/// checklist, gives a checklist that copies the SeedQR and no words
/// (`docs/BACKUP-FLOW.md` §19.21 item 11).
#[test]
fn change_the_plan_from_the_checklist_shows_the_board() {
    let mut app = fresh_three();
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    app.press(Action::BPlan);
    let b = app.backup.as_ref().unwrap();
    assert_eq!(b.stage, BStage::Plan);
    assert_eq!(b.q, Some(qstep::BOARD), "the board card is open");
    let _ = app.frame();
    assert!(app.offers(Action::Board(BP::Customize)), "still read-only");
    app.press(Action::Board(BP::Customize));
    app.press(Action::Board(BP::Form(Zone::Paper(0), Thing::Seed(0), 1)));
    app.press(Action::Board(BP::Form(Zone::Paper(0), Thing::Seed(0), 0)));
    app.press(Action::BQNext(qstep::BOARD));
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    assert_eq!(
        sorted(paper(&app, 0)),
        sorted(vec![What::SeedQr(0), What::Sheet])
    );
    assert!(app.backup_items().contains(&bstep::of(Item::Copy(0))));
}

/// A new plan keeps each seed's words on paper and no SeedQR, a single key
/// and a 2-of-3 alike; Customize backup and the SeedQR's toggle add it
/// (`docs/BACKUP-FLOW.md` §19.21 item 12).
#[test]
fn a_new_plan_keeps_the_words_and_no_seedqr() {
    let mut single = testkit::started();
    let fp = key_in(&mut single, 1);
    single.press(Action::KeyWallet(fp, 1));
    single.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    let w = single.session.wallets.len() - 1;
    single.press(Action::Backup(w));
    let three = fresh_three();
    for (mut app, n) in [(single, 1), (three, 3)] {
        for p in 0..n {
            assert_eq!(paper(&app, p), vec![What::Words(p), What::Sheet]);
        }
        app.press(Action::BChecklist);
        let items = app.backup_items();
        assert!(items.contains(&bstep::of(Item::Copy(0))), "{items:?}");
        app.press(Action::BPlan);
        app.press(Action::Board(BP::Customize));
        app.press(Action::Board(BP::Form(Zone::Paper(0), Thing::Seed(0), 1)));
        assert_eq!(
            sorted(paper(&app, 0)),
            sorted(vec![What::Words(0), What::SeedQr(0), What::Sheet])
        );
    }
}

/// What a box holds, in an order of its own: the board keeps what was
/// added last at the end.
fn sorted(mut v: Vec<What>) -> Vec<String> {
    let mut out: Vec<String> = v.drain(..).map(|w| format!("{w:?}")).collect();
    out.sort();
    out
}
