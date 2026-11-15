//! The backup's plan on its own: each preset fills the answers the
//! wallet's shape calls for, the check says what one place lost or found
//! gives, the checklist lists only what the plan needs, and the answers
//! read back as the vault keeps them.

use crate::common::{
    board, files_at, no_paper_description, pass_off_at, put, take_off, vault_at, vault_made,
    vault_stick, words_off_at,
};
use faraday_core::Faraday;
use faraday_core::board::{BoardPress, Chip, Thing, Zone};
use faraday_core::plan::{
    Answers, At, Form, Found, Item, Lost, Note, Preset, Question, Row, Seed, Shape, Spot, What,
    check, checklist, form, place_map, seeds, software, wallet,
};

fn seed(name: &str, here: bool) -> Seed {
    Seed {
        name: faraday_text::Text::of(name),
        here,
        shares: false,
        passphrase: false,
    }
}

/// What a plan keeps nowhere, which Make the checklist and a changed plan
/// in Lock and write both ask, says a seed here past the checklist's 28
/// numbers: no checklist or flow is made that would mix two seeds' items.
#[test]
fn a_seed_past_the_checklists_numbers_is_kept_nowhere() {
    let c = faraday_core::testkit::compute();
    let seeds: Vec<Seed> = (0..29)
        .map(|i| seed(&format!("{:08x}", 0x1000_0000u32 + i), i == 28))
        .collect();
    let shape = Shape {
        paths: Vec::new(),
        m: 2,
        keys: 29,
        seeds,
        splits: true,
        vaults: 0,
    };
    let a = Answers::default();
    let line = faraday_core::plan::kept_nowhere(&c, &shape, &a, &[28]);
    assert!(
        line.as_deref().is_some_and(|l| l.contains("28")),
        "{line:?}"
    );
}

fn one_key() -> Shape {
    Shape {
        paths: Vec::new(),
        m: 1,
        keys: 1,
        seeds: vec![seed("aaaa0001", true)],
        splits: false,
        vaults: 0,
    }
}

fn three_of_five() -> Shape {
    Shape {
        paths: Vec::new(),
        m: 3,
        keys: 5,
        seeds: (1..=5)
            .map(|i| seed(&format!("aaaa000{i}"), true))
            .collect(),
        splits: true,
        vaults: 0,
    }
}

fn two_of_three() -> Shape {
    Shape {
        paths: Vec::new(),
        m: 2,
        keys: 3,
        seeds: vec![
            seed("aaaa0001", true),
            seed("aaaa0002", true),
            seed("aaaa0003", true),
        ],
        splits: true,
        vaults: 0,
    }
}

#[test]
fn each_preset_fills_a_one_key_wallets_answers() {
    let c = faraday_core::testkit::compute();
    let shape = one_key();
    for preset in Preset::ALL {
        let a = Answers::preset(&shape, preset);
        let vault = !matches!(preset, Preset::Paper | Preset::Standard);
        assert!(a.seeds[seeds::WORDS], "{preset:?}: paper words");
        // The words and no SeedQR, The standard way too (§19.21 item 12).
        assert!(!a.seeds[seeds::SEEDQR], "{preset:?}");
        assert!(!a.seeds[seeds::FILE], "{preset:?}");
        assert_eq!(a.seeds[seeds::VAULT], vault, "{preset:?}");
        // One place for a single key, whatever the preset (§19.19).
        assert_eq!(a.places, 1, "{preset:?}: places");
        assert!(a.wallet[wallet::PAPER], "{preset:?}: the sheet in each");
        assert_eq!(a.wallet[wallet::VAULT], vault, "{preset:?}");
        assert_eq!(
            a.wallet[wallet::SOFTWARE],
            preset == Preset::PaperVaultSoftware,
            "{preset:?}"
        );
        assert!(a.form[form::QR] && !a.form[form::TEXT], "{preset:?}: as QR");
        // The sheet in each place, the words in each.
        for spot in place_map(&c, &shape, &a)
            .iter()
            .filter(|s| matches!(s.at, At::Place(_)))
        {
            let holds: Vec<What> = spot.holds.iter().map(|(w, _)| *w).collect();
            assert!(holds.contains(&What::Sheet), "{preset:?} {spot:?}");
            assert!(holds.contains(&What::Words(0)), "{preset:?} {spot:?}");
            assert!(!holds.contains(&What::SeedQr(0)), "{preset:?} {spot:?}");
        }
    }
}

#[test]
fn each_preset_fills_a_two_of_threes_answers() {
    let c = faraday_core::testkit::compute();
    let shape = two_of_three();
    for preset in Preset::ALL {
        let a = Answers::preset(&shape, preset);
        assert_eq!(a.places, 3, "{preset:?}: a place per key");
        assert!(!a.split, "{preset:?}: the whole sheet in each");
        assert_eq!(
            a.seeds[seeds::VAULT],
            !matches!(preset, Preset::Paper | Preset::Standard),
            "{preset:?}"
        );
        let places: Vec<_> = place_map(&c, &shape, &a)
            .into_iter()
            .filter(|s| matches!(s.at, At::Place(_)))
            .collect();
        assert_eq!(places.len(), 3);
        for (p, spot) in places.iter().enumerate() {
            let holds: Vec<What> = spot.holds.iter().map(|(w, _)| *w).collect();
            assert!(holds.contains(&What::Words(p)), "{preset:?}: its seed");
            assert!(!holds.contains(&What::SeedQr(p)), "{preset:?}: no SeedQR");
            assert!(holds.contains(&What::Sheet), "{preset:?}: the whole sheet");
            assert!(
                !holds.iter().any(|w| matches!(w, What::Share(_))),
                "{preset:?}: no share"
            );
        }
    }
}

#[test]
fn seeds_held_elsewhere_get_no_paper_here_and_watch_only_asks_nothing_of_seeds() {
    let c = faraday_core::testkit::compute();
    let mut shape = two_of_three();
    shape.seeds[1].here = false;
    shape.seeds[2].here = false;
    let a = Answers::preset(&shape, Preset::Paper);
    let boxes = place_map(&c, &shape, &a);
    let away = boxes
        .iter()
        .find(|s| s.at == At::Away)
        .expect("on its own device");
    assert_eq!(away.holds.len(), 2);
    assert!(!boxes.iter().any(|s| {
        s.holds
            .iter()
            .any(|(w, _)| matches!(w, What::Words(1) | What::Words(2)))
    }));
    for s in &mut shape.seeds {
        s.here = false;
    }
    let a = Answers::preset(&shape, Preset::PaperVault);
    assert!(!a.paper_seeds() && !a.seeds[seeds::VAULT]);
    assert!(!checklist(&c, &shape, &a).iter().any(|i| match i {
        Item::Templates | Item::Copy(_) => true,
        Item::Vault(v) => !a.vault_seeds(&shape, *v).is_empty(),
        _ => false,
    }));
}

#[test]
fn one_place_with_paper_only_does_not_survive_losing_it() {
    let c = faraday_core::testkit::compute();
    let shape = one_key();
    let mut a = Answers::preset(&shape, Preset::Paper);
    a.toggle(&shape, Question::Places, 1);
    assert_eq!(a.places, 1);
    let c = check(&c, &shape, &a);
    assert_eq!(c.lost, Lost::No);
    assert_eq!(c.spend, Found::Yes);
}

#[test]
fn three_places_with_split_shares_survive_any_one_lost() {
    let c = faraday_core::testkit::compute();
    let shape = two_of_three();
    let whole = Answers::preset(&shape, Preset::Paper);
    // The whole sheet in each place: one place found sees the balance.
    assert_eq!(check(&c, &shape, &whole).balance, Found::Yes);
    let mut a = whole.clone();
    a.toggle(&shape, Question::Split, 1);
    assert!(a.split, "Its own share is the second row");
    let c = check(&c, &shape, &a);
    assert_eq!(c.lost, Lost::Yes);
    // No one place holds two seeds, nor every key.
    assert_eq!(c.spend, Found::No);
    assert_eq!(c.balance, Found::No);
}

#[test]
fn a_place_with_a_quorum_of_seeds_on_paper_can_spend() {
    let c = faraday_core::testkit::compute();
    let shape = two_of_three();
    let mut a = Answers::preset(&shape, Preset::Paper);
    a.toggle(&shape, Question::Places, 2);
    // Place 1 keeps seeds 1 and 3, with shares 1 and 3, which hold every
    // key.
    assert_eq!(a.words_at(&shape, 0), vec![0, 2]);
    assert_eq!(check(&c, &shape, &a).spend, Found::Yes);
}

#[test]
fn the_vault_counts_with_its_passphrase() {
    let c = faraday_core::testkit::compute();
    let shape = one_key();
    // Two places, as a plan saved before one place per key (§19.19).
    let mut a = Answers::preset_at(&shape, Preset::PaperVault, 2);
    // No paper seed: the seed is in the vault alone, its stick in place 2.
    a.toggle(&shape, Question::Seeds, seeds::WORDS);
    a.toggle(&shape, Question::Sticks(0), 0);
    let found = check(&c, &shape, &a);
    assert_eq!(
        found.lost,
        Lost::WithVault,
        "the vault's sticks are in both places"
    );
    assert_eq!(found.spend, Found::OnlyWithVault);
    // One stick, in place 2: losing it loses the seed.
    a.toggle(&shape, Question::Sticks(0), 0);
    assert_eq!(check(&c, &shape, &a).lost, Lost::No);
}

#[test]
fn a_passphrase_never_shares_a_place_with_its_words() {
    let c = faraday_core::testkit::compute();
    let mut shape = two_of_three();
    shape.seeds[0].passphrase = true;
    let mut a = Answers::preset(&shape, Preset::Paper);
    assert!(!a.pass_at(0, 0), "not with its words");
    assert!(a.pass_at(0, 1), "the next place keeps it");
    a.toggle(&shape, Question::Passphrase(0), 0);
    assert!(!a.pass_at(0, 0), "a tick beside its words is refused");
    // Its words without it rebuild nothing: losing place 2 leaves seed 1
    // without its passphrase, and seed 3 alone.
    assert_eq!(check(&c, &shape, &a).lost, Lost::No);
}

#[test]
fn the_checklist_lists_only_what_the_plan_needs() {
    let c = faraday_core::testkit::compute();
    let shape = two_of_three();
    let paper = checklist(&c, &shape, &Answers::preset(&shape, Preset::Paper));
    assert_eq!(
        paper,
        vec![
            Item::Templates,
            Item::Copy(0),
            Item::Copy(1),
            Item::Copy(2),
            Item::Sheets,
            Item::Envelopes,
            Item::Sticks
        ]
    );
    let mut a = Answers::preset(&shape, Preset::PaperVaultSoftware);
    let all = checklist(&c, &shape, &a);
    for item in [
        Item::Vault(0),
        Item::Vault(1),
        Item::Vault(2),
        Item::ShowDescriptor,
    ] {
        assert!(all.contains(&item), "{item:?}");
    }
    assert!(
        !all.contains(&Item::PublicFiles),
        "QR only: nothing on a stick"
    );
    a.toggle(&shape, Question::Form, form::TEXT);
    a.toggle(&shape, Question::Software, software::SPARROW);
    assert!(checklist(&c, &shape, &a).contains(&Item::PublicFiles));
}

#[test]
fn the_answers_read_back_as_the_vault_keeps_them() {
    let mut shape = two_of_three();
    shape.seeds[2].passphrase = true;
    let mut a = Answers::preset(&shape, Preset::PaperVaultSoftware);
    a.toggle(&shape, Question::Seeds, seeds::SEEDQR);
    a.toggle(&shape, Question::Software, software::CORE);
    let text = a.to_text();
    assert_eq!(Answers::from_text(&shape, &text), Some(a));
    // Not for another shape.
    assert_eq!(Answers::from_text(&one_key(), &text), None);
}

/// A vault whose plan record was crafted to name more places than the
/// wallet could have, or a seed it does not have, opens with the plan read
/// as absent.
#[test]
fn a_plan_naming_what_the_wallet_does_not_have_is_read_as_absent() {
    let shape = two_of_three();
    let a = Answers::preset(&shape, Preset::PaperVaultSoftware);
    let text = String::from(&*a.to_text());
    assert_eq!(Answers::from_text(&shape, &text), Some(a.clone()));
    let places = format!("places {}\n", a.places);
    let huge = text.replace(&places, "places 1000000000\n");
    assert_ne!(huge, text);
    assert_eq!(Answers::from_text(&shape, &huge), None);
    assert_eq!(
        Answers::from_text(&shape, &format!("{text}part 0 99\n")),
        None
    );
    // A vault or stick row numbered as high as a count goes reads as
    // absent, in a build that checks its sums as in one that does not.
    let row = shape.seeds.len();
    let last = format!("vault {} {}\n", usize::MAX, "0".repeat(row));
    assert_eq!(Answers::from_text(&shape, &format!("{text}{last}")), None);
    let last = format!("sticks {} {}\n", usize::MAX, "0".repeat(a.places));
    assert_eq!(Answers::from_text(&shape, &format!("{text}{last}")), None);
    // A note on a seed the wallet has is kept, on one it does not, not.
    assert!(Note::from_text("exposed place0 seed2", 3).is_some());
    assert!(Note::from_text("exposed place0 seed99", 3).is_none());
}

/// A plan holds 128 vaults: a saved plan with a 129th vault row is read as
/// absent, as one with seeds past the checklist's numbers is kept nowhere.
#[test]
fn a_plan_with_a_129th_vault_is_read_as_absent() {
    let shape = two_of_three();
    let a = Answers::preset(&shape, Preset::PaperVaultSoftware);
    let text = String::from(&*a.to_text());
    let row = "0".repeat(shape.seeds.len());
    let last = format!("vault 127 {row}\n");
    assert!(Answers::from_text(&shape, &format!("{text}{last}")).is_some());
    let past = format!("vault 128 {row}\n");
    assert_eq!(Answers::from_text(&shape, &format!("{text}{past}")), None);
    // A vault past 128 named by a spot, by a vault's stick, or by a
    // stick row is refused too.
    let mut b = a.clone();
    let n = b.add_vault(&shape);
    b.add_vault_stick(&shape, 0, n);
    let kept = String::from(&*b.to_text());
    assert_eq!(Answers::from_text(&shape, &kept).as_ref(), Some(&b));
    for (from, to) in [
        (format!("vault{n}"), "vault128"),
        (format!("stick{n}"), "stick128"),
    ] {
        assert!(kept.contains(&from), "{kept}");
        let past = kept.replace(&from, to);
        assert_eq!(Answers::from_text(&shape, &past), None, "{to}");
    }
    let rows = format!("{text}sticks 127 {}\n", "0".repeat(a.places));
    assert!(Answers::from_text(&shape, &rows).is_some());
    let rows = format!("{text}sticks 128 {}\n", "0".repeat(a.places));
    assert_eq!(Answers::from_text(&shape, &rows), None);
}

/// A plan a test build saved with access layers (`layers` and `route`
/// lines, a vault's password on paper in a place) opens as the same plan
/// without them: the layers were withdrawn (F38), the plan is not refused.
#[test]
fn a_plan_saved_with_access_layers_opens_without_them() {
    let shape = two_of_three();
    let a = Answers::preset(&shape, Preset::PaperVaultSoftware);
    let mut text = a.to_text();
    text.push_str("layers 2\nroute 0 2 0,1 - - paper\nroute 1 1 2 0 -\n");
    assert_eq!(Answers::from_text(&shape, &text), Some(a));
}

/// A 2-of-3 made in Create over the three test keys, its backup opened
/// from Create's Back up card with every software chosen and the
/// description going as files: the public
/// files item offers each file Create's Public files card offered, each
/// named for the wallet.
#[test]
fn the_public_files_item_offers_the_files_create_offered_for_the_wallet() {
    let c = faraday_core::testkit::compute();
    use faraday_core::create::NewKind;
    use faraday_core::{Action, bstep, cstep, qrow, testkit};
    use faraday_shell_api::{App, Event, Key};
    let mut app = testkit::started();
    for word in ["bacon", "zebra", "summer"] {
        app.press(Action::Entry(None));
        for c in testkit::test_words(word).chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        app.press(Action::EntryAdd);
        faraday_core::testkit::keep_published(&mut app);
    }
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, multi);
    for slot in 0..3u8 {
        let fp = app.session.keys[usize::from(slot)].fingerprint.0;
        app.press(Action::CSlotHere(slot, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let built = app.create().and_then(|c| c.built).expect("made");
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, Preset::PaperVaultSoftware);
    for row in 0..5u8 {
        if !app.backup().unwrap().answers.software[usize::from(row)] {
            app.press(Action::BAnswer(qrow::SOFTWARE, row));
        }
    }
    // The software takes the description's text too: the public files
    // item. (The description as files at a place is written with that
    // place's stick, no item of its own: `docs/BACKUP-FLOW.md` §17.12
    // item 6.)
    app.press(Action::BAnswer(qrow::FORM, form::TEXT as u8));
    // Nothing by hand or into a vault, so the item is open to reach.
    crate::common::no_words(&mut app);
    crate::common::no_vaults(&mut app);
    app.press(Action::BChecklist);
    assert!(
        app.backup_items_now(&c).contains(&bstep::PUBLIC),
        "the checklist has the public files item"
    );
    app.press(Action::BStep(bstep::PUBLIC));
    // Making the checklist made its files in the text form too: each
    // taken back out, to be made from the item.
    for n in [1u8, 2, 5, 6, 7] {
        app.press(Action::PublicRemove(built, n));
    }
    // Down the column to the item's rows, past a vault item per seed.
    for _ in 0..10 {
        let _ = app.frame();
        if app.offers(Action::PublicOut(built, 1)) {
            break;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 400,
            dy: 200,
        });
    }
    assert!(
        app.offers(Action::PublicOut(built, 1)),
        "the item is open on the descriptor"
    );
    let stem = faraday_core::file_stem(&app.session.wallets[built].name);
    let stem = stem.as_str();
    // Descriptor, multisig config, wallet file, BSMS record, Core import.
    let files = [
        (1u8, "descriptor.txt"),
        (2, "multisig-config.txt"),
        (5, "wallet.json"),
        (6, "bsms.txt"),
        (7, "bitcoin-core.json"),
    ];
    let offered = app.backup_public();
    for (n, end) in files {
        assert!(offered.contains(&n), "the item offers {end}");
        app.press(Action::PublicOut(built, n));
        let name = format!("{stem}-{end}");
        assert!(
            app.outbox.iter().any(|f| f.name.as_str() == name.as_str()),
            "{name} reached the Outbox"
        );
    }
}

/// A single-key wallet made in Create from a key whose New key was given
/// a passphrase: its plan asks where the passphrase goes,
/// on each place's card.
#[test]
fn a_single_key_create_with_a_passphrase_asks_where_the_passphrase_goes() {
    use faraday_core::Action;
    let mut app = passphrase_key();
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert!(app.backup().is_some(), "the backup is open");
    // The board offers the passphrase, and each place's paper takes it.
    open_board(&mut app);
    assert!(shows(
        &mut app,
        Action::Board(BoardPress::Chip(Chip::List(Thing::Pass(0))))
    ));
    for p in 0..app.backup().unwrap().answers.places {
        assert!(
            shows(&mut app, Action::Board(BoardPress::Zone(Zone::Paper(p)))),
            "Place {}'s paper",
            p + 1
        );
    }
}

/// Create on the test network with a single key made by New key, given a
/// passphrase, open on its Back up card.
fn passphrase_key() -> Faraday {
    use faraday_core::keygen::Way;
    use faraday_core::{Action, cstep};
    use faraday_shell_api::{App, Event, Key};
    let mut app = faraday_core::testkit::started();
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 5 + i / 3) % 2 == 0));
    }
    app.press(Action::KNext);
    for field in [0, 1] {
        app.press(Action::KPassField(field));
        for c in "Ride the 7 bus".chars() {
            app.event(Event::Key(Key::Char(c)));
        }
    }
    app.press(Action::KLock);
    app.press(Action::KNext);
    let c = app.create().expect("create");
    assert_eq!(c.open, Some(cstep::CHECK));
    app
}

// ---------------------------------------------------------------------
// The plan leads with the whole wallet sheet;
// shares come second, with a guide page and a slider.
// ---------------------------------------------------------------------

/// An app on the test network with Savings (2-of-3) and a 3-of-5 loaded,
/// and the backup of wallet `name` open on its Places question with the
/// Paper preset.
fn places_of(name: &str) -> faraday_core::Faraday {
    let c = faraday_core::testkit::compute();
    use faraday_core::{Action, qstep, testkit};
    use faraday_shell_api::App;
    let mut app = testkit::started();
    app.session = testkit::session();
    app.session
        .add_wallet(
            &c,
            "Savings",
            &testkit::savings(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.session
        .add_wallet(
            &c,
            "Five",
            &testkit::three_of_five(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let w = app
        .session
        .wallets
        .iter()
        .position(|w| w.name == name)
        .unwrap();
    app.press(Action::Backup(w));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    // The preset opens the board itself, opened for changes.
    if app.backup().unwrap().q != Some(qstep::BOARD) {
        app.press(Action::BQ(qstep::BOARD));
    }
    app.press(Action::Board(faraday_core::board::BoardPress::Customize));
    let _ = app.frame();
    app
}

/// A multisig's plan keeps the whole wallet sheet in each place; the
/// board's list offers **Add partial descriptions to this backup** and no
/// share by itself (`docs/BACKUP-FLOW.md` §17.19 item 1).
#[test]
fn a_fresh_multisig_plan_keeps_the_whole_sheet_in_each_place() {
    use faraday_core::Action;
    use faraday_shell_api::App;
    let mut app = places_of("Savings");
    let b = app.backup().unwrap();
    assert!(!b.answers.split);
    let shape = app.plan_shape_now(b.wallet);
    for p in 0..b.answers.places {
        assert!(b.answers.sheet_at(&shape, p), "place {p}: the whole sheet");
        assert!(b.answers.shares_at(&shape, p).is_empty(), "place {p}");
    }
    let _ = app.frame();
    assert!(shows(&mut app, Action::Board(BoardPress::Partials)));
    for j in 0..3 {
        assert!(
            !shows(
                &mut app,
                Action::Board(BoardPress::Chip(Chip::List(Thing::Share(j))))
            ),
            "Share {} by itself",
            j + 1
        );
    }
}

/// A 3-of-5 over five places, partial descriptions added: each share
/// leaves 2 keys off, any 3 places rebuild the description, and no place
/// sees the balance once the whole is off every place
/// (`docs/BACKUP-FLOW.md` §17.19 items 2 and 6). The checklist's Shares
/// card still sets the keys left off.
#[test]
fn a_three_of_five_over_five_places_leaves_two_keys_off_each_share() {
    let c = faraday_core::testkit::compute();
    use faraday_core::seeds::SLIDE_OMIT;
    use faraday_core::{Action, bstep};
    use faraday_shell_api::App;
    let shape = three_of_five();
    let mut a = Answers::preset(&shape, Preset::Standard);
    assert_eq!(a.places, 5);
    let out = a.add_partials(&c, &shape);
    assert!(out.rebuilds && out.sees.is_empty(), "{out:?}");
    assert_eq!(a.omit, 2);
    a.whole_off_places(&shape);
    assert_partials_hold(&shape, &a);
    // On the board: the button, then the checklist's Shares card.
    let mut app = places_of("Five");
    app.press(Action::Board(BoardPress::Partials));
    assert_eq!(app.sheet, Some(faraday_core::Sheet::BoardPartials));
    app.press(Action::Board(BoardPress::PartialsWholeOff));
    assert_eq!(app.backup().unwrap().answers.omit, 2);
    app.press(Action::BQ(faraday_core::qstep::PLAN));
    app.press(Action::BChecklist);
    app.press(Action::BStep(bstep::SHEETS));
    let _ = app.frame();
    assert!(app.offers(Action::Slide(SLIDE_OMIT, 1)), "the Shares card");
    app.press(Action::Slide(SLIDE_OMIT, 1));
    assert_eq!(app.backup().unwrap().answers.omit, 1);
}

/// What is a share? opens the guide on the page about shares; closing it
/// leaves the plan on the board.
#[test]
fn what_is_a_share_opens_the_page_and_back_returns_to_places() {
    use faraday_core::{Action, Screen, Sheet, qstep};
    let mut app = places_of("Savings");
    assert!(shows(&mut app, Action::GuideShares));
    app.press(Action::GuideShares);
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert_eq!(app.guide().shown, "backup", "the guide at the place");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup().unwrap().q, Some(qstep::BOARD));
}

/// A 2-of-3 with seeds 1 and 2 typed in here and the third a cosigner's.
fn two_here() -> Shape {
    let mut shape = two_of_three();
    shape.seeds[2].here = false;
    shape
}

/// Which seeds whoever finds place `p` alone holds a copy of: its words
/// and SeedQRs, and the seeds in each vault whose stick it keeps.
fn keys_at(shape: &Shape, a: &Answers, p: usize) -> Vec<usize> {
    let c = faraday_core::testkit::compute();
    let boxes = place_map(&c, shape, a);
    let mut out: Vec<usize> = Vec::new();
    let place = boxes.iter().find(|s| s.at == At::Place(p)).unwrap();
    for (what, _) in &place.holds {
        let seeds: Vec<usize> = match *what {
            What::Words(i) | What::SeedQr(i) => vec![i],
            What::VaultStick(v) => boxes
                .iter()
                .find(|s| s.at == At::Vault(v))
                .unwrap()
                .holds
                .iter()
                .filter_map(|(w, _)| match w {
                    What::Seed(i) | What::SeedPassphrase(i) => Some(*i),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        out.extend(seeds);
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn sticks_at(shape: &Shape, a: &Answers, p: usize) -> Vec<usize> {
    let c = faraday_core::testkit::compute();
    place_map(&c, shape, a)
        .into_iter()
        .filter(|s| s.at == At::Place(p))
        .flat_map(|s| s.holds)
        .filter_map(|(w, _)| match w {
            What::VaultStick(v) => Some(v),
            _ => None,
        })
        .collect()
}

#[test]
fn paper_and_vault_puts_one_seed_in_each_vault_each_stick_away_from_the_other_seeds_words() {
    let c = faraday_core::testkit::compute();
    let shape = two_here();
    let a = Answers::preset(&shape, Preset::PaperVault);
    let vaults: Vec<(At, Vec<What>)> = place_map(&c, &shape, &a)
        .into_iter()
        .filter(|s| matches!(s.at, At::Vault(_)))
        .map(|s| (s.at, s.holds.into_iter().map(|(w, _)| w).collect()))
        .collect();
    assert_eq!(
        vaults,
        vec![
            (At::Vault(0), vec![What::Seed(0), What::Wallet]),
            (At::Vault(1), vec![What::Seed(1), What::Wallet]),
        ],
        "a vault per seed here, the description in each"
    );
    // Vault 1's stick at Place 3, which keeps no words; Vault 2's at
    // Place 2, beside its own seed's words.
    assert_eq!(sticks_at(&shape, &a, 0), Vec::<usize>::new());
    assert_eq!(sticks_at(&shape, &a, 1), vec![1]);
    assert_eq!(sticks_at(&shape, &a, 2), vec![0]);
    assert_eq!(
        place_map(&c, &shape, &a)[2]
            .holds
            .iter()
            .find(|(w, _)| matches!(w, What::VaultStick(_)))
            .map(|(w, _)| w.label(&shape, faraday_core::Medium::Stick.cap())),
        Some(faraday_text::Text::of("Stick with Vault 1"))
    );
    // So no one place found spends, even with a vault's passphrase; any
    // one lost leaves the rest to rebuild it.
    let found = check(&c, &shape, &a);
    assert_eq!(found.spend, Found::No);
    assert_eq!(found.lost, Lost::Yes);
    assert_eq!(
        checklist(&c, &shape, &a)
            .into_iter()
            .filter(|i| matches!(i, Item::Vault(_)))
            .collect::<Vec<_>>(),
        vec![Item::Vault(0), Item::Vault(1)]
    );
}

#[test]
fn a_seed_ticked_into_a_second_vault_is_kept_in_both() {
    let shape = two_here();
    let mut a = Answers::preset(&shape, Preset::PaperVault);
    a.toggle(&shape, Question::Vault(0), 1);
    assert_eq!(a.vault_seeds(&shape, 0), vec![0, 1]);
    assert_eq!(a.vault_seeds(&shape, 1), vec![1]);
    assert_eq!(a.vaults_made(&shape), vec![0, 1]);
    // The cosigner's seed is not here: it goes into no vault.
    a.toggle(&shape, Question::Vault(0), 2);
    assert_eq!(a.vault_seeds(&shape, 0), vec![0, 1]);
    // A vault with no seed is not made.
    a.toggle(&shape, Question::Vault(1), 1);
    assert_eq!(a.vaults_made(&shape), vec![0]);
    let text = a.to_text();
    assert_eq!(Answers::from_text(&shape, &text), Some(a));
}

#[test]
fn no_place_holds_two_different_keys_where_the_places_allow_it() {
    let five = Shape {
        paths: Vec::new(),
        m: 3,
        keys: 5,
        seeds: (1..=5)
            .map(|k| seed(&format!("aaaa000{k}"), k <= 3))
            .collect(),
        splits: true,
        vaults: 0,
    };
    for (shape, name) in [
        (one_key(), "one key"),
        (two_here(), "2-of-3, two here"),
        (five, "3-of-5, three here"),
    ] {
        let a = Answers::preset(&shape, Preset::PaperVault);
        for v in a.vaults_made(&shape) {
            assert!(
                (0..a.places).any(|p| a.stick_at(v, p)),
                "{name}: vault {v} has a place"
            );
        }
        for p in 0..a.places {
            let keys = keys_at(&shape, &a, p);
            assert!(keys.len() <= 1, "{name}: place {p} holds {keys:?}");
        }
    }
    // A single key: its one place keeps its words and the vault's stick
    // (§19.19).
    let shape = one_key();
    let a = Answers::preset(&shape, Preset::PaperVault);
    assert_eq!(a.places, 1);
    assert_eq!(a.words_at(&shape, 0), vec![0]);
    assert_eq!(sticks_at(&shape, &a, 0), vec![0]);
}

#[test]
fn a_plan_saved_with_one_vault_still_loads() {
    let c = faraday_core::testkit::compute();
    let shape = two_of_three();
    // As plans were kept before a vault per seed: one `sticks` line, no
    // `vault` line.
    let old =
        "seeds 1010\nplaces 3\nsplit 0\nsticks 100\nwallet 1100\nsoftware 00001\nform 10\nomit 1\n";
    let a = Answers::from_text(&shape, old).expect("the old plan loads");
    assert_eq!(a.vaults_made(&shape), vec![0], "one vault");
    assert_eq!(a.vault_seeds(&shape, 0), vec![0, 1, 2], "every seed here");
    assert_eq!(sticks_at(&shape, &a, 0), vec![0], "its stick at Place 1");
    assert!(sticks_at(&shape, &a, 1).is_empty() && sticks_at(&shape, &a, 2).is_empty());
    let vault = place_map(&c, &shape, &a)
        .into_iter()
        .find(|s| s.at == At::Vault(0))
        .unwrap();
    assert_eq!(vault.holds.len(), 4, "three seeds and the wallet");
}

/// A single key whose words are unticked keeps its only copy in Vault 1,
/// and Vault 1's stick still has a place (`docs/WALLETS.md` §13);
/// with one place for two vaults, both sticks go there.
#[test]
fn the_single_key_with_words_unticked_places_vault_1s_stick() {
    let shape = one_key();
    let mut a = Answers::preset(&shape, Preset::PaperVault);
    a.toggle(&shape, Question::Seeds, seeds::WORDS);
    assert!(a.words_at(&shape, 0).is_empty() && a.words_at(&shape, 1).is_empty());
    assert!((0..a.places).any(|p| a.stick_at(0, p)), "Vault 1's stick");
    // From Paper only: the words unticked, then the vault ticked.
    let mut a = Answers::preset(&shape, Preset::Paper);
    a.toggle(&shape, Question::Seeds, seeds::WORDS);
    a.toggle(&shape, Question::Seeds, seeds::VAULT);
    assert!((0..a.places).any(|p| a.stick_at(0, p)), "Vault 1's stick");
    // Two vaults and one place: both sticks are kept there.
    let shape = two_here();
    let mut a = Answers::preset(&shape, Preset::PaperVault);
    a.toggle(&shape, Question::Places, 1);
    assert_eq!(a.places, 1);
    for v in a.vaults_made(&shape) {
        assert!(a.stick_at(v, 0), "vault {v} has no place");
    }
}

/// With the check reading No, the map panel says why: the seed kept only
/// in a vault whose one stick is at the place lost.
#[test]
fn a_plan_that_does_not_survive_a_place_lost_says_why() {
    let c = faraday_core::testkit::compute();
    let shape = one_key();
    // Two places, as a plan saved before one place per key (§19.19).
    let mut a = Answers::preset_at(&shape, Preset::PaperVault, 2);
    a.toggle(&shape, Question::Seeds, seeds::WORDS);
    let stick = (0..a.places).find(|&p| a.stick_at(0, p)).unwrap();
    assert_eq!(check(&c, &shape, &a).lost, Lost::No);
    let name = |p: usize| faraday_text::text!("Place {}", p + 1);
    assert_eq!(
        faraday_core::plan::lost_why(&c, &shape, &a, &name, "stick").as_deref(),
        Some(
            format!(
                "Seed aaaa0001 is kept only in Vault 1, whose stick is at Place {}",
                stick + 1
            )
            .as_str()
        )
    );
    // Paper and vault as the preset makes it: nothing to say.
    let a = Answers::preset(&shape, Preset::PaperVault);
    assert_eq!(
        faraday_core::plan::lost_why(&c, &shape, &a, &name, "stick"),
        None
    );
}

/// A vault the board made, left with no stick place, is out of The plan
/// and holds nothing back (`docs/BACKUP-FLOW.md` §17.16 item 3): Make the
/// checklist is offered with no line about it, and the checklist makes
/// no vault; its stick kept at a place again, it is in the plan again.
#[test]
fn a_vault_with_no_stick_place_is_out_of_the_plan() {
    let c = faraday_core::testkit::compute();
    use faraday_core::plan::Item;
    use faraday_core::{Action, qstep};
    let mut app = places_of("Savings");
    let w = app.backup().unwrap().wallet;
    let shape = app.plan_shape_now(w);
    // Vault 1 at Place 3 with the description in it, then Place 3 gone.
    // No seed of Savings is here: the vault's row ticks at once.
    assert!(shape.watch_only());
    vault_made(&mut app, 0);
    put(&mut app, Thing::Vault(0), Zone::Card(2));
    let s = vault_stick(&app, 2, 0).expect("its stick at Place 3");
    put(&mut app, Thing::Whole, Zone::Vault(2, s, 0));
    let a = app.backup().unwrap().answers.clone();
    assert_eq!(a.vaults_made(&shape), vec![0]);
    board(&mut app, BoardPress::RemovePlace(2));
    let a = app.backup().unwrap().answers.clone();
    assert!(a.vaults_made(&shape).is_empty(), "out of The plan");
    assert!(!faraday_core::plan::checklist(&c, &shape, &a).contains(&Item::Vault(0)));
    app.press(Action::BQ(qstep::PLAN));
    assert!(shows(&mut app, Action::BChecklist));
    assert!(!shows_text(&mut app, "Vault 1's stick needs a place"));
    // Its stick kept at a place again, it is in the plan again.
    open_board(&mut app);
    board(&mut app, BoardPress::AddPlace);
    put(&mut app, Thing::Vault(0), Zone::Card(2));
    let a = app.backup().unwrap().answers.clone();
    assert_eq!(a.vaults_made(&shape), vec![0]);
    assert!(faraday_core::plan::checklist(&c, &shape, &a).contains(&Item::Vault(0)));
}

// ---------------------------------------------------------------------
// `docs/BACKUP-FLOW.md` §15: the plan asked place by place
// ---------------------------------------------------------------------

/// Whether the plan offers `action` anywhere down its column, scrolled
/// to as a person would.
fn shows(app: &mut Faraday, action: faraday_core::Action) -> bool {
    use faraday_shell_api::{App, Event};
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        if app.offers(action) {
            return true;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    false
}

/// Whether a text with `part` in it is drawn anywhere down the column.
fn shows_text(app: &mut Faraday, part: &str) -> bool {
    use faraday_shell_api::{App, Event};
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        if app.drawn_texts().iter().any(|t| t.contains(part)) {
            return true;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    false
}

/// Opens the board's card, unless it is open.
fn open_board(app: &mut Faraday) {
    use faraday_core::{Action, qstep};
    use faraday_shell_api::App;
    if app.backup().unwrap().q != Some(qstep::BOARD) {
        app.press(Action::BQ(qstep::BOARD));
    }
    // Opened for changes (`docs/BACKUP-FLOW.md` §17.6).
    app.press(Action::Board(faraday_core::board::BoardPress::Customize));
    let _ = app.frame();
}

/// The plan's map as the cards show it: each place with what its
/// sticks keep.
fn the_map(app: &Faraday) -> Vec<Spot> {
    let c = faraday_core::testkit::compute();
    let b = app.backup().unwrap();
    place_map(&c, &app.plan_shape_now(b.wallet), &b.answers)
}

/// What spot `at` holds on the map.
fn held(app: &Faraday, at: At) -> Vec<What> {
    the_map(app)
        .into_iter()
        .filter(|s| s.at == at)
        .flat_map(|s| s.holds.into_iter().map(|(w, _)| w))
        .collect()
}

/// A 2-of-3 made in Create over three keys loaded here, its backup open
/// on Paper only: three places.
fn three_here() -> Faraday {
    use faraday_core::create::NewKind;
    use faraday_core::{Action, cstep, testkit};
    use faraday_shell_api::{App, Event, Key};
    let mut app = testkit::started();
    for word in ["bacon", "zebra", "summer"] {
        app.press(Action::Entry(None));
        for c in testkit::test_words(word).chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        app.press(Action::EntryAdd);
        faraday_core::testkit::keep_published(&mut app);
    }
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, multi);
    for slot in 0..3u8 {
        let fp = app.session.keys[usize::from(slot)].fingerprint.0;
        app.press(Action::CSlotHere(slot, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert_eq!(app.backup().unwrap().answers.places, 3);
    app
}

/// The vaults on the map, each with the seeds it holds, and the places
/// that keep each one's stick.
fn vaults_of(app: &Faraday) -> Vec<(usize, Vec<What>, Vec<usize>)> {
    let m = the_map(app);
    m.iter()
        .filter_map(|s| match s.at {
            At::Vault(v) => Some((
                v,
                s.holds.iter().map(|(w, _)| *w).collect(),
                m.iter()
                    .filter_map(|p| match p.at {
                        At::Place(n) if p.holds.iter().any(|(w, _)| *w == What::VaultStick(v)) => {
                            Some(n)
                        }
                        _ => None,
                    })
                    .collect(),
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn each_place_has_its_card_and_a_vaults_sheet_puts_the_seeds_ticked_in_it() {
    use faraday_core::Action;
    let mut app = three_here();
    open_board(&mut app);
    let card = |p: usize| Action::Board(BoardPress::Zone(Zone::Card(p)));
    for p in 0..3 {
        assert!(shows(&mut app, card(p)), "Place {}'s card", p + 1);
    }
    assert!(!shows(&mut app, card(3)), "three places, three cards");
    // Each vault dropped on its place, the seeds dropped into it.
    vault_at(&mut app, 0, 0, &[0, 1]);
    vault_at(&mut app, 1, 1, &[2]);
    assert_eq!(
        vaults_of(&app),
        vec![
            (0, vec![What::Seed(0), What::Seed(1)], vec![0]),
            (1, vec![What::Seed(2)], vec![1]),
        ]
    );
}

#[test]
fn words_at_a_place_are_on_the_map_and_its_card_says_what_finding_it_alone_gives() {
    let c = faraday_core::testkit::compute();
    use faraday_core::plan::{Found, alone};
    use faraday_shell_api::App;
    let mut app = three_here();
    open_board(&mut app);
    // Place 3's paper keeps the first seed's words in place of its own.
    take_off(&mut app, Zone::Paper(2), Thing::Seed(2));
    put(&mut app, Thing::Seed(0), Zone::Paper(2));
    let words: Vec<What> = held(&app, At::Place(2))
        .into_iter()
        .filter(|w| matches!(w, What::Words(_)))
        .collect();
    assert_eq!(
        words,
        vec![What::Words(0)],
        "Place 3 keeps the first seed's words"
    );
    let b = app.backup().unwrap();
    let found = alone(&c, &app.plan_shape_now(b.wallet), &b.answers, At::Place(2));
    // One seed of a 2-of-3 and the whole sheet: the balance, not a spend.
    assert_eq!((found.spend, found.balance), (Found::No, Found::Yes));
    let _ = app.frame();
    assert!(
        app.drawn_texts().iter().any(|t| t == found.text()),
        "{:?}",
        app.drawn_texts()
    );
}

/// A chip picked up and let go without a zone leaves the map as it was.
#[test]
fn done_with_no_seed_ticked_leaves_the_row_unticked_and_the_map_as_it_was() {
    let c = faraday_core::testkit::compute();
    use faraday_core::Action;
    let mut app = three_here();
    open_board(&mut app);
    let before = the_map(&app);
    let chip = Action::Board(BoardPress::Chip(Chip::List(Thing::Seed(0))));
    app.press(chip);
    assert!(app.backup().unwrap().board.held.is_some());
    app.press(chip);
    assert_eq!(app.backup().unwrap().board.held, None);
    assert_eq!(the_map(&app), before);
    let b = app.backup().unwrap();
    assert_eq!(
        b.answers
            .row_seeds(&c, &app.plan_shape_now(b.wallet), 2, Row::SeedQr),
        None
    );
}

/// What goes into a vault is the person's choice: a vault dropped on a
/// place keeps its stick there, holding nothing yet; the description can
/// then go into it, and until something does, the vault holds The plan.
#[test]
fn a_vault_with_no_seed_keeps_its_stick_and_holds_the_plan_until_it_holds_something() {
    let c = faraday_core::testkit::compute();
    use faraday_core::{Action, BStage, qstep};
    let mut app = three_here();
    open_board(&mut app);
    vault_made(&mut app, 0);
    put(&mut app, Thing::Vault(0), Zone::Card(2));
    let b = app.backup().unwrap();
    let shape = app.plan_shape_now(b.wallet);
    assert_eq!(
        b.answers.row_seeds(&c, &shape, 2, Row::Vault(0)),
        Some(Vec::new())
    );
    assert!(held(&app, At::Place(2)).contains(&What::VaultStick(0)));
    assert!(
        shows_text(&mut app, "Vault 1 holds nothing"),
        "the board's line says so"
    );
    app.press(Action::BQ(qstep::PLAN));
    assert!(!shows(&mut app, Action::BChecklist));
    assert!(shows_text(&mut app, "Vault 1 holds nothing"));
    app.press(Action::BChecklist);
    assert_eq!(app.backup().unwrap().stage, BStage::Plan);
    // The description into it, at Place 3.
    open_board(&mut app);
    let s = vault_stick(&app, 2, 0).expect("its stick at Place 3");
    let into = Action::Board(BoardPress::Zone(Zone::Vault(2, s, 0)));
    assert!(shows(&mut app, into));
    put(&mut app, Thing::Whole, Zone::Vault(2, s, 0));
    assert_eq!(held(&app, At::Vault(0)), vec![What::Wallet]);
    app.press(Action::BQ(qstep::PLAN));
    assert!(shows(&mut app, Action::BChecklist));
}

/// A seed's words and SeedQR put where its passphrase is kept are kept
/// there, the passphrase too, and the board says so: nothing is refused
/// for being unwise (`docs/BACKUP-FLOW.md` §17.5).
#[test]
fn words_cannot_be_ticked_where_the_passphrase_is_kept() {
    use faraday_core::Action;
    let mut app = passphrase_key();
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    open_board(&mut app);
    // Paper only: the words at Place 1, the passphrase alone at Place 2.
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert!(held(&app, At::Place(1)).contains(&What::Passphrase(0)));
    put(&mut app, Thing::Seed(0), Zone::Paper(1));
    crate::common::form(&mut app, Zone::Paper(1), Thing::Seed(0), 1);
    let here = held(&app, At::Place(1));
    assert!(here.contains(&What::Passphrase(0)), "kept");
    assert!(here.contains(&What::Words(0)) && here.contains(&What::SeedQr(0)));
    assert!(shows_text(&mut app, "words with its passphrase"));
}

/// A plan saved before the cards that keeps the seeds and the
/// description on the stick of files shows them on the board's software
/// card, the stick of public files, each to take off; the last taken off
/// takes the stick of files off the plan.
#[test]
fn an_old_plans_stick_of_files_shows_under_not_a_place_and_can_be_emptied() {
    use faraday_core::Action;
    let mut app = three_here();
    let w = app.backup().unwrap().wallet;
    let shape = app.plan_shape_now(w);
    let old =
        "seeds 1001\nplaces 3\nsplit 0\nsticks 000\nwallet 1001\nsoftware 00001\nform 10\nomit 1\n";
    let a = Answers::from_text(&shape, old).expect("the old plan loads");
    assert!(the_files(&shape, &a) > 0);
    faraday_core::testkit::set_backup_answers(&mut app, a);
    open_board(&mut app);
    let n = the_files(&shape, &app.backup().unwrap().answers);
    assert_eq!(n, 4, "three seeds and the description");
    let things = [Thing::Seed(0), Thing::Seed(1), Thing::Seed(2), Thing::Whole];
    for t in things {
        let off = Action::Board(BoardPress::Off(Zone::SoftStick, t));
        assert!(shows(&mut app, off), "{t:?}");
        app.press(off);
    }
    assert!(the_map(&app).iter().all(|s| s.at != At::Files));
}

/// What the old stick of files holds.
fn the_files(shape: &Shape, a: &Answers) -> usize {
    let c = faraday_core::testkit::compute();
    a.old_files(&c, shape).len()
}

/// Change the plan from the checklist opens the board on the map the
/// cards made, nothing replaced (§19.21 item 11).
#[test]
fn change_the_plan_from_the_checklist_opens_the_place_cards_with_the_map_kept() {
    use faraday_core::{Action, BStage, qstep};
    let mut app = three_here();
    vault_at(&mut app, 0, 0, &[0, 1]);
    let before = the_map(&app);
    app.press(Action::BQ(qstep::PLAN));
    app.press(Action::BChecklist);
    assert_eq!(app.backup().unwrap().stage, BStage::Checklist);
    assert!(shows(&mut app, Action::BPlan));
    app.press(Action::BPlan);
    assert_eq!(app.sheet, None, "nothing is asked");
    assert_eq!(app.backup().unwrap().stage, BStage::Plan);
    assert_eq!(the_map(&app), before);
    assert_eq!(app.backup().unwrap().q, Some(qstep::BOARD));
    let s = vault_stick(&app, 0, 0).expect("Vault 1's stick at Place 1");
    assert!(shows(
        &mut app,
        Action::Board(BoardPress::Off(Zone::Files(0, s), Thing::Vault(0)))
    ));
}

#[test]
fn a_vault_kept_at_two_places_is_one_vault_with_two_sticks() {
    use faraday_core::Action;
    let mut app = three_here();
    open_board(&mut app);
    vault_at(&mut app, 0, 0, &[0]);
    put(&mut app, Thing::Vault(0), Zone::Card(2));
    // What it holds is drawn at Place 3 too; a second seed dropped into
    // it there is in the vault at both places.
    let s = vault_stick(&app, 2, 0).expect("its stick at Place 3");
    assert!(shows(
        &mut app,
        Action::Board(BoardPress::Chip(Chip::Placed(
            Zone::Vault(2, s, 0),
            Thing::Seed(0)
        )))
    ));
    put(&mut app, Thing::Seed(1), Zone::Vault(2, s, 0));
    assert_eq!(
        vaults_of(&app),
        vec![(0, vec![What::Seed(0), What::Seed(1)], vec![0, 2])]
    );
}

/// A passphrase put where its words are is kept there, with the board's
/// warning line naming the place: nothing is refused for being unwise
/// (`docs/BACKUP-FLOW.md` §17.5).
#[test]
fn a_passphrase_cannot_be_ticked_where_its_words_are() {
    let c = faraday_core::testkit::compute();
    use faraday_core::Action;
    let mut app = passphrase_key();
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    open_board(&mut app);
    let b = app.backup().unwrap();
    let shape = app.plan_shape_now(b.wallet);
    assert_eq!(
        b.answers.row_seeds(&c, &shape, 0, Row::Words),
        Some(vec![0])
    );
    put(&mut app, Thing::Pass(0), Zone::Paper(0));
    assert!(held(&app, At::Place(0)).contains(&What::Passphrase(0)));
    let fp = String::from(&*shape.seeds[0].name);
    assert!(shows_text(
        &mut app,
        &format!("Place 1 keeps {fp}'s words with its passphrase")
    ));
}

/// A 2-of-3 over three places, partial descriptions added: one share at
/// each place, each leaving 1 key off, no place seeing the balance, any 2
/// places rebuilding the description; Take it off every place leaves the
/// description in shares alone and any 2 still rebuild it; Remove partial
/// descriptions takes every share off; the hand-out is the same on every
/// run (`docs/BACKUP-FLOW.md` §17.19 item 6).
#[test]
fn in_shares_a_share_on_paper_at_a_place_is_on_the_map_there() {
    let c = faraday_core::testkit::compute();
    let shape = two_of_three();
    let mut a = Answers::preset(&shape, Preset::Standard);
    let out = a.add_partials(&c, &shape);
    assert_eq!(a.omit, 1);
    assert!(out.rebuilds && out.sees.is_empty(), "{out:?}");
    let mut seen: Vec<usize> = Vec::new();
    for p in 0..3 {
        let shares = a.shares_at(&shape, p);
        assert_eq!(shares.len(), 1, "Place {}: one share", p + 1);
        assert!(!seen.contains(&shares[0]), "each its own");
        seen.push(shares[0]);
        assert!(a.sheet_at(&shape, p), "the whole stays until taken off");
    }
    // The same on every run.
    let mut again = Answers::preset(&shape, Preset::Standard);
    assert_eq!(again.add_partials(&c, &shape), out);
    assert_eq!(again, a);
    a.whole_off_places(&shape);
    assert_eq!(a.whole_at(&c, &shape), Vec::<usize>::new());
    assert_partials_hold(&shape, &a);
    assert_eq!(check(&c, &shape, &a).lost, Lost::Yes);
    a.remove_partials(&shape);
    for p in 0..3 {
        assert!(a.shares_at(&shape, p).is_empty(), "Place {}", p + 1);
    }
    assert!(!a.partials_on());
}

/// A 2-of-3 over seventeen places, partial descriptions added: three
/// shares over seventeen places leave some two places holding the same
/// share, so the hand-out does not say the plan rebuilds, and the board
/// offers Keep it only, never Take it off every place.
#[test]
fn above_sixteen_places_a_hand_out_is_not_called_rebuilding_unchecked() {
    let c = faraday_core::testkit::compute();
    let shape = two_of_three();
    let mut a = Answers::preset(&shape, Preset::Standard);
    while a.places < 17 {
        a.add_place(&shape);
    }
    assert_eq!(a.places, 17);
    let out = a.add_partials(&c, &shape);
    assert!(!out.rebuilds, "{out:?}");
}

/// A 2-of-3 over two places, one holding two seeds: that place spends
/// and sees; the other, with one seed, does not see the balance from its
/// share; both together rebuild the description.
#[test]
fn a_two_of_three_over_two_places_keeps_the_rules_with_one_holding_two_seeds() {
    let c = faraday_core::testkit::compute();
    let shape = two_of_three();
    let mut a = Answers::preset(&shape, Preset::Standard);
    // Seed 3 moved to Place 1, Place 3 taken off.
    a.ensure_map(&shape);
    let m = faraday_core::plan::stored_map(&shape, &a);
    let m = faraday_core::plan::with_added(&m, At::Place(0), What::Words(2));
    a.set_map(m);
    a.remove_place(&shape, 2);
    assert_eq!(a.places, 2);
    let out = a.add_partials(&c, &shape);
    assert!(out.rebuilds, "{out:?}");
    a.whole_off_places(&shape);
    let alone = faraday_core::plan::alone(&c, &shape, &a, At::Place(1));
    assert_ne!(alone.balance, Found::Yes, "Place 2 does not see alone");
    assert!(rebuilds_at(&shape, &a, &[0]), "Place 1 spends, so rebuilds");
    assert!(rebuilds_at(&shape, &a, &[0, 1]));
}

/// The keys every group of `m` places, and every group holding `m` seeds,
/// rebuild; no place that cannot spend sees the balance.
fn assert_partials_hold(shape: &Shape, a: &Answers) {
    let c = faraday_core::testkit::compute();
    let places = a.places;
    for g in 1u32..(1 << places) {
        let group: Vec<usize> = (0..places).filter(|p| g & (1 << p) != 0).collect();
        let seeds: usize = (0..shape.seeds.len())
            .filter(|&i| group.iter().any(|&p| a.words_at(shape, p).contains(&i)))
            .count();
        if group.len() >= shape.m || seeds >= shape.m {
            assert!(rebuilds_at(shape, a, &group), "{group:?} rebuilds");
        }
    }
    for p in 0..places {
        let alone = faraday_core::plan::alone(&c, shape, a, At::Place(p));
        if alone.spend != Found::Yes {
            assert_ne!(alone.balance, Found::Yes, "Place {} sees alone", p + 1);
        }
    }
}

/// Whether places `group` together put the description back together:
/// the whole of it, or its shares and their own seeds holding every key.
fn rebuilds_at(shape: &Shape, a: &Answers, group: &[usize]) -> bool {
    let split = faraday_core::wallet_backup::split_plan(shape.keys, shape.m, a.omit);
    (0..shape.keys).all(|k| {
        group.iter().any(|&p| {
            a.sheet_at(shape, p)
                || a.words_at(shape, p).contains(&k)
                || a.shares_at(shape, p)
                    .iter()
                    .any(|&j| split.get(j).is_some_and(|row| row.contains(&k)))
        })
    })
}

/// No preset makes more places than one per key, and the board offers no
/// **+ place** at that many (`docs/BACKUP-FLOW.md` §17.19).
#[test]
fn no_preset_makes_more_than_one_place_per_key() {
    use faraday_core::Action;
    use faraday_shell_api::App;
    for shape in [one_key(), two_of_three(), three_of_five()] {
        for preset in Preset::ALL {
            let a = Answers::preset(&shape, preset);
            assert!(
                a.places <= faraday_core::plan::max_places(&shape),
                "{preset:?}: {} places for {} keys",
                a.places,
                shape.keys
            );
        }
    }
    let mut app = places_of("Savings");
    assert_eq!(app.backup().unwrap().answers.places, 3);
    let _ = app.frame();
    assert!(!shows(&mut app, Action::Board(BoardPress::AddPlace)));
    app.press(Action::Board(BoardPress::AddPlace));
    assert_eq!(app.backup().unwrap().answers.places, 3, "none added");
    board(&mut app, BoardPress::RemovePlace(2));
    assert!(shows(&mut app, Action::Board(BoardPress::AddPlace)));
}

#[test]
fn a_seed_file_at_a_place_has_that_places_stick_of_files_and_is_held_until_written() {
    let c = faraday_core::testkit::compute();
    use faraday_bip::keys::Fingerprint;
    use faraday_core::{Action, bstep};
    let mut app = passphrase_key();
    let fp = app.session.keys.last().unwrap().fingerprint;
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    // The seed as a file at Place 2, its passphrase at Place 1; nothing
    // else is kept.
    files_at(&mut app, 1, &[0]);
    words_off_at(&mut app, 0);
    words_off_at(&mut app, 1);
    pass_off_at(&mut app, 1, 0);
    put(&mut app, Thing::Pass(0), Zone::Paper(0));
    no_paper_description(&mut app);
    assert_eq!(held(&app, At::Place(1)), vec![What::SeedFile(0)]);
    app.press(Action::BChecklist);
    // The file is made and written with its stick: no item of its own
    // (`docs/BACKUP-FLOW.md` §17.12 item 6). The passphrase on paper has
    // its own (`docs/WALLETS.md` §13).
    assert_eq!(
        app.backup_items_now(&c),
        vec![bstep::PASSPHRASE, bstep::ENVELOPE, bstep::STICKS]
    );
    let rows = app.wallet_stick_rows_now(&c);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].0, "Files (Place 2)");
    let name = format!(
        "seed-{}.txt",
        faraday_core::wallet::fp_text(Fingerprint(fp.0)).to_lowercase()
    );
    assert_eq!(rows[0].1, vec![faraday_text::Text::of(&name)], "{rows:?}");
    app.press(Action::BNext(bstep::ENVELOPE));
    assert!(
        app.key_held(&c, Fingerprint(fp.0)),
        "the key waits for its file's stick"
    );
}

#[test]
fn fewer_places_take_the_last_with_what_it_held_and_a_seed_kept_nowhere_holds_the_plan() {
    use faraday_core::{Action, BStage, qstep};
    let mut app = passphrase_key();
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    // The words at Place 1, the passphrase alone at Place 2 (§19.19).
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert_eq!(app.backup().unwrap().answers.places, 2);
    // The seed's words at Place 2 alone, its passphrase at Place 1.
    put(&mut app, Thing::Seed(0), Zone::Paper(1));
    words_off_at(&mut app, 0);
    put(&mut app, Thing::Pass(0), Zone::Paper(0));
    // Place 2's ×: it goes with what it kept.
    board(&mut app, BoardPress::RemovePlace(1));
    assert_eq!(app.backup().unwrap().answers.places, 1);
    assert!(the_map(&app).iter().all(|s| s.at != At::Place(1)));
    assert!(
        !the_map(&app)
            .iter()
            .any(|s| s.holds.iter().any(|(w, _)| *w == What::Words(0)))
    );
    app.press(Action::BQ(qstep::PLAN));
    assert!(!shows(&mut app, Action::BChecklist));
    app.press(Action::BQ(qstep::PLAN));
    app.press(Action::BQ(qstep::PLAN));
    assert!(shows_text(&mut app, "is kept nowhere"));
    app.press(Action::BChecklist);
    assert_eq!(app.backup().unwrap().stage, BStage::Plan);
}

#[test]
fn make_the_checklist_is_on_the_plan_card_alone_and_the_checklist_is_the_maps() {
    let c = faraday_core::testkit::compute();
    use faraday_core::{Action, bstep, qstep};
    let mut app = three_here();
    vault_at(&mut app, 0, 0, &[0, 1]);
    for q in app.backup_questions(&c) {
        app.press(Action::BQ(q));
        if app.backup().unwrap().q != Some(q) {
            app.press(Action::BQ(q));
        }
        assert_eq!(
            shows(&mut app, Action::BChecklist),
            q == qstep::PLAN,
            "card {q}"
        );
    }
    let b = app.backup().unwrap();
    let want: Vec<u8> = checklist(&c, &app.plan_shape_now(b.wallet), &b.answers)
        .into_iter()
        .map(bstep::of)
        .collect();
    app.press(Action::BChecklist);
    assert_eq!(app.backup_items_now(&c), want);
    assert!(
        app.backup_items_now(&c)
            .contains(&bstep::of(Item::Vault(0)))
    );
}

/// A single key with a passphrase may have a second place for the
/// passphrase alone: the standard way makes it, + place is hidden at 2 for
/// that key and at 1 for a key without a passphrase, and a multisig whose
/// seeds have passphrases still caps at its keys (`docs/BACKUP-FLOW.md`
/// §17.19).
#[test]
fn a_single_key_with_a_passphrase_has_a_second_place_for_it() {
    use faraday_core::Action;
    let mut app = passphrase_key();
    app.press(Action::CBackup);
    let w = app.backup().unwrap().wallet;
    let shape = app.plan_shape_now(w);
    assert_eq!(faraday_core::plan::max_places(&shape), 2);
    let a = &app.backup().unwrap().answers;
    assert_eq!(a.places, 2);
    assert!(
        a.pass_at(0, 1) && !a.pass_at(0, 0),
        "the passphrase alone at Place 2"
    );
    assert!(
        !shows(&mut app, Action::Board(BoardPress::AddPlace)),
        "none at 2"
    );
    board(&mut app, BoardPress::RemovePlace(1));
    assert!(
        shows(&mut app, Action::Board(BoardPress::AddPlace)),
        "offered at 1"
    );
    board(&mut app, BoardPress::AddPlace);
    assert_eq!(app.backup().unwrap().answers.places, 2);
    assert!(!shows(&mut app, Action::Board(BoardPress::AddPlace)));
}

#[test]
fn passphrases_add_no_place_to_a_key_without_one_or_to_a_multisig() {
    use faraday_core::plan::max_places;
    assert_eq!(max_places(&one_key()), 1);
    let mut multi = two_of_three();
    for s in &mut multi.seeds {
        s.passphrase = true;
    }
    assert_eq!(max_places(&multi), 3);
    for preset in Preset::ALL {
        assert!(Answers::preset(&multi, preset).places <= 3, "{preset:?}");
    }
}

#[test]
fn each_preset_fills_the_board() {
    let c = faraday_core::testkit::compute();
    use faraday_core::Action;
    // Each plan as a person now makes it from The standard way (§19.21).
    for preset in Preset::ALL.iter() {
        let mut app = passphrase_key();
        app.press(Action::CBackup);
        faraday_core::testkit::backup_as(&mut app, *preset);
        let w = app.backup().unwrap().wallet;
        let shape = app.plan_shape_now(w);
        let a = &app.backup().unwrap().answers;
        assert_eq!(
            a.row_seeds(&c, &shape, 0, Row::Words),
            Some(vec![0]),
            "{preset:?}"
        );
        // Two places for a key with a passphrase (§19.19): the seed at
        // Place 1, the vault's stick there too, the passphrase alone at
        // Place 2.
        assert_eq!(a.places, 2, "{preset:?}: two places");
        assert!(a.pass_at(0, 1) && !a.pass_at(0, 0), "{preset:?}");
        let vault = !matches!(preset, Preset::Paper | Preset::Standard);
        assert_eq!(
            a.row_seeds(&c, &shape, 0, Row::Vault(0)).is_some(),
            vault,
            "{preset:?}: Vault 1's stick at Place 1"
        );
        assert_eq!(
            a.software_on(&c, &shape),
            *preset == Preset::PaperVaultSoftware,
            "{preset:?}"
        );
    }
}

#[test]
fn a_plan_saved_before_the_cards_shows_on_them_as_its_map() {
    let c = faraday_core::testkit::compute();
    let shape = two_of_three();
    let old =
        "seeds 1010\nplaces 3\nsplit 0\nsticks 100\nwallet 1100\nsoftware 00001\nform 10\nomit 1\n";
    let a = Answers::from_text(&shape, old).expect("the old plan loads");
    assert!(a.map.is_none(), "no spot lines");
    assert_eq!(
        a.row_seeds(&c, &shape, 0, Row::Vault(0)),
        Some(vec![0, 1, 2])
    );
    assert_eq!(a.row_seeds(&c, &shape, 1, Row::Vault(0)), None);
    for p in 0..3 {
        assert_eq!(a.row_seeds(&c, &shape, p, Row::Words), Some(vec![p]));
        assert!(a.form_at(&c, &shape, p, Form::Paper), "the sheet at {p}");
    }
    // A card's first answer makes that map the plan, as it was.
    let mut b = a.clone();
    b.ensure_map(&shape);
    assert_eq!(place_map(&c, &shape, &b), place_map(&c, &shape, &a));
    assert_eq!(Answers::from_text(&shape, &b.to_text()), Some(b));
}

/// How near a place brings a spend is a search over the other places,
/// bounded: a plan of 26 places, each keeping one of 26 seeds, 13 to
/// sign, is past the bound, and the place's tag says the spend was not
/// worked out rather than a count it did not reach.
#[test]
fn a_reach_past_the_searchs_bound_says_it_was_not_worked_out() {
    let c = faraday_core::testkit::compute();
    let seeds: Vec<faraday_core::plan::Seed> = (0..26)
        .map(|i| faraday_core::plan::Seed {
            name: faraday_text::text!("{:08x}", 0x1000_0000u32 + i),
            here: true,
            shares: false,
            passphrase: false,
        })
        .collect();
    let shape = faraday_core::plan::Shape {
        paths: Vec::new(),
        m: 13,
        keys: 26,
        seeds,
        splits: true,
        vaults: 0,
    };
    let mut a = faraday_core::plan::Answers::default();
    a.set_map(
        (0..26)
            .map(|i| Spot {
                at: At::Place(i),
                holds: vec![(What::Words(i), faraday_core::plan::Tag::Secret)],
            })
            .collect(),
    );
    let r = faraday_core::plan::reach(&c, &shape, &a, 0).expect("the place keeps a seed");
    let (text, _) = r.tag();
    assert!(text.contains("not worked out"), "{text}");
}

/// A 2-of-2 sortedmulti wallet read with its keys in the other order is
/// one wallet: the plan saved for it opens with its places, notes and
/// holdings on the same keys, and saving it again leaves one plan in the
/// vault.
#[test]
fn a_plan_saved_for_a_wallet_opens_for_the_same_wallet_with_its_keys_in_the_other_order() {
    use crate::chart_edit::{desktop, fresh_vault};
    use crate::common::RecordsOf as _;
    use faraday_core::glance::{Backup, BackupNode};
    use faraday_core::plan::{Mark, Tag};
    use faraday_core::{Action, testkit};
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let c = testkit::compute();
    let path = "m/48'/1'/0'/2'";
    let (x, y) = (testkit::key(0, path), testkit::key(1, path));
    let descriptor = |a: &str, b: &str| format!("wsh(sortedmulti(2,{a}/<0;1>/*,{b}/<0;1>/*))");
    let mut app = desktop();
    for n in 0..2 {
        app.press(faraday_core::Action::Entry(None));
        crate::common::type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        app.press(faraday_core::Action::EntryAdd);
    }
    fresh_vault(&mut app);
    let load = |app: &mut Faraday, text: String| {
        app.session.wallets.clear();
        app.session
            .add_wallet(
                &c,
                "Pair",
                &text,
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap()
    };
    let xy_text = descriptor(&x, &y);
    let w = load(&mut app, xy_text.clone());
    let xy = app.session.wallets[w].policy().clone();
    // The plan for X, Y: the words of Y (seed 1) at Place 1 marked lost,
    // those of X (seed 0) at Place 2 checked.
    let shape = app.plan_shape_now(w);
    let mut answers = Answers::preset(&shape, Preset::Standard);
    answers.places = 2;
    answers.custom = true;
    let spot = |p: usize, i: usize| Spot {
        at: At::Place(p),
        holds: vec![(What::Words(i), Tag::Secret)],
    };
    answers.map = Some(vec![spot(0, 1), spot(1, 0)]);
    let mut record = Record::new(kind::PLAN)
        .with(field::PLAN_WALLET, xy.to_descriptor().as_bytes())
        .with(field::PLAN_ANSWERS, answers.to_text().as_bytes());
    for note in [
        Note::Marked(At::Place(0), What::Words(1), Mark::Lost),
        Note::Checked(
            At::Place(1),
            What::Words(0),
            faraday_text::Text::of("2026-10-10"),
        ),
    ] {
        record.push(field::PLAN_NOTE, note.to_text().as_bytes());
    }
    let id = app.vaults().open[0].id;
    faraday_core::testkit::vault_edit(&mut app, id, |s| s.insert(id, record));
    let node = |app: &Faraday, w: usize, name: &str| -> BackupNode {
        let c = testkit::compute();
        match faraday_core::glance::of_now(&c, app, w).unwrap().backup {
            Backup::Plan { nodes, .. } => nodes
                .into_iter()
                .find(|n| n.name == name)
                .unwrap_or_else(|| panic!("no {name} on the chart")),
            other => panic!("no plan on the chart: {other:?}"),
        }
    };
    // As saved: Y is key 2, X is key 1.
    let first = node(&app, w, "Place 1");
    assert_eq!(first.holds, ["Key 2 words"]);
    assert_eq!(first.marks, [Some(Mark::Lost)]);
    assert_eq!(node(&app, w, "Place 2").holds, ["Key 1 words"]);
    assert_eq!(
        node(&app, w, "Place 2").checked,
        [Some(faraday_text::Text::of("2026-10-10"))]
    );
    // The same wallet with its keys the other way: Y is key 1 now, and
    // the plan is on the same keys.
    let w = load(&mut app, descriptor(&y, &x));
    let place_1 = node(&app, w, "Place 1");
    assert_eq!(place_1.holds, ["Key 1 words"]);
    assert_eq!(place_1.marks, [Some(Mark::Lost)]);
    let place_2 = node(&app, w, "Place 2");
    assert_eq!(place_2.holds, ["Key 2 words"]);
    assert_eq!(
        place_2.checked,
        [Some(faraday_text::Text::of("2026-10-10"))]
    );
    // Saved again, it is still the one plan.
    app.press(Action::Backup(w));
    let shape = app.plan_shape_now(w);
    testkit::set_backup_answers(&mut app, Answers::preset(&shape, Preset::Standard));
    app.press(Action::BChecklist);
    let plans = app
        .vaults()
        .store()
        .contents_for_tests(id)
        .of(kind::PLAN)
        .count();
    assert_eq!(plans, 1, "one plan record for the wallet");
}

/// A plan with a split saved for one key order does not open for the
/// other: a share holds the keys of its window, so it is not moved; the
/// same plan without a split still opens (the test above).
#[test]
fn a_plan_with_shares_saved_for_one_key_order_does_not_open_for_the_other() {
    use crate::chart_edit::{desktop, fresh_vault};
    use faraday_core::glance::Backup;
    use faraday_core::plan::Tag;
    use faraday_core::testkit;
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let c = testkit::compute();
    let path = "m/48'/1'/0'/2'";
    let (x, y) = (testkit::key(0, path), testkit::key(1, path));
    let descriptor = |a: &str, b: &str| format!("wsh(sortedmulti(2,{a}/<0;1>/*,{b}/<0;1>/*))");
    let mut app = desktop();
    for n in 0..2 {
        app.press(faraday_core::Action::Entry(None));
        crate::common::type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        app.press(faraday_core::Action::EntryAdd);
    }
    fresh_vault(&mut app);
    let load = |app: &mut Faraday, text: String| {
        app.session.wallets.clear();
        app.session
            .add_wallet(
                &c,
                "Pair",
                &text,
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap()
    };
    let w = load(&mut app, descriptor(&x, &y));
    let xy = app.session.wallets[w].policy().clone();
    let shape = app.plan_shape_now(w);
    let mut answers = Answers::preset(&shape, Preset::Standard);
    answers.places = 2;
    answers.split = true;
    answers.custom = true;
    answers.map = Some(vec![
        Spot {
            at: At::Place(0),
            holds: vec![(What::Share(0), Tag::Secret)],
        },
        Spot {
            at: At::Place(1),
            holds: vec![(What::Share(1), Tag::Secret)],
        },
    ]);
    let record = Record::new(kind::PLAN)
        .with(field::PLAN_WALLET, xy.to_descriptor().as_bytes())
        .with(field::PLAN_ANSWERS, answers.to_text().as_bytes());
    let id = app.vaults().open[0].id;
    faraday_core::testkit::vault_edit(&mut app, id, |s| s.insert(id, record));
    let holds = |app: &Faraday, w: usize| -> Vec<String> {
        match faraday_core::glance::of_now(&testkit::compute(), app, w)
            .unwrap()
            .backup
        {
            Backup::Plan { nodes, .. } => nodes
                .into_iter()
                .flat_map(|n| n.holds)
                .map(|h| h.to_string())
                .collect(),
            _ => Vec::new(),
        }
    };
    // For the keys as they were, the shares are on the chart.
    assert!(
        holds(&app, w).iter().any(|h| h.contains("hare")),
        "{:?}",
        holds(&app, w)
    );
    // For the keys the other way, the plan is not read.
    let w = load(&mut app, descriptor(&y, &x));
    assert!(
        holds(&app, w).iter().all(|h| !h.contains("hare")),
        "{:?}",
        holds(&app, w)
    );
}

/// An app holding seeds `seeds` and a vault with a plan with shares and
/// `notes` saved for the sortedmulti wallet of keys `a`, `b`, loaded: the
/// app and the vault's id.
fn shares_plan_kept(
    seeds: &[usize],
    a: &str,
    b: &str,
    notes: &[Note],
) -> (Faraday, faraday_keys::vault::VaultId) {
    use crate::chart_edit::{desktop, fresh_vault};
    use faraday_core::plan::Tag;
    use faraday_core::testkit;
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let c = testkit::compute();
    let mut app = desktop();
    for &n in seeds {
        app.press(faraday_core::Action::Entry(None));
        crate::common::type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        app.press(faraday_core::Action::EntryAdd);
    }
    fresh_vault(&mut app);
    let text = format!("wsh(sortedmulti(2,{a}/<0;1>/*,{b}/<0;1>/*))");
    let w = app
        .session
        .add_wallet(
            &c,
            "Pair",
            &text,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let policy = app.session.wallets[w].policy().clone();
    let shape = app.plan_shape_now(w);
    let mut answers = Answers::preset(&shape, Preset::Standard);
    answers.places = 2;
    answers.split = true;
    answers.custom = true;
    answers.map = Some(vec![
        Spot {
            at: At::Place(0),
            holds: vec![(What::Share(0), Tag::Secret)],
        },
        Spot {
            at: At::Place(1),
            holds: vec![(What::Share(1), Tag::Secret)],
        },
    ]);
    let mut record = Record::new(kind::PLAN)
        .with(field::PLAN_WALLET, policy.to_descriptor().as_bytes())
        .with(field::PLAN_ANSWERS, answers.to_text().as_bytes());
    for note in notes {
        record.push(field::PLAN_NOTE, note.to_text().as_bytes());
    }
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| s.insert(id, record));
    (app, id)
}

#[test]
fn a_note_saved_from_the_other_key_orders_chart_leaves_the_shares_plans_notes_as_they_were() {
    use crate::chart_edit::node_of;
    use crate::common::RecordsOf;
    use faraday_core::glance::Press;
    use faraday_core::glance::sheet::ChartAction as C;
    use faraday_core::plan::Mark;
    use faraday_core::testkit;
    use faraday_vault::records::{field, kind};
    let c = testkit::compute();
    let path = "m/48'/1'/0'/2'";
    let (x, y) = (testkit::key(0, path), testkit::key(1, path));
    let notes = [
        Note::Marked(At::Place(0), What::Share(0), Mark::Lost),
        Note::Checked(
            At::Place(1),
            What::Share(1),
            faraday_text::Text::of("2026-10-10"),
        ),
    ];
    let (mut app, id) = shares_plan_kept(&[0, 1], &x, &y, &notes);
    let kept = |app: &Faraday| -> Vec<Vec<u8>> {
        let all = app.vaults().store().contents_for_tests(id);
        all.of(kind::PLAN)
            .take(1)
            .flat_map(|(_, r)| {
                faraday_keys::vault::RecordView::of(r)
                    .public_all(field::PLAN_NOTE)
                    .map(<[u8]>::to_vec)
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    let before = kept(&app);
    assert_eq!(before.len(), 2);
    // The wallet with its keys the other way: a mark made on its chart.
    app.session.wallets.clear();
    let text = format!("wsh(sortedmulti(2,{y}/<0;1>/*,{x}/<0;1>/*))");
    let w = app
        .session
        .add_wallet(
            &c,
            "Pair",
            &text,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    // A plan of its own is kept beside it, which the chart shows.
    let shape = app.plan_shape_now(w);
    let own = faraday_vault::Record::new(kind::PLAN)
        .with(
            field::PLAN_WALLET,
            app.session.wallets[w].policy().to_descriptor().as_bytes(),
        )
        .with(
            field::PLAN_ANSWERS,
            Answers::preset(&shape, Preset::Standard)
                .to_text()
                .as_bytes(),
        );
    testkit::vault_edit(&mut app, id, |s| s.insert(id, own));
    let press = Press::Loaded(w);
    let (n, _) = node_of(&app, w, "Place 1");
    app.press(faraday_core::Action::Chart(C::Mark(
        press,
        n,
        None,
        Mark::Exposed,
    )));
    assert_eq!(kept(&app), before);
    // The press was not a no-op: the wallet's own plan, the second record,
    // took the mark (written once for the wallet and once for its sheet).
    let all = app.vaults().store().contents_for_tests(id);
    let own_notes: Vec<String> = all
        .of(kind::PLAN)
        .nth(1)
        .map(|(_, r)| {
            faraday_keys::vault::RecordView::of(r)
                .public_all(field::PLAN_NOTE)
                .map(|n| String::from_utf8_lossy(n).into_owned())
                .collect()
        })
        .unwrap();
    assert_eq!(own_notes.len(), 2);
    assert!(own_notes.iter().all(|n| n.starts_with("exposed place0 ")));
}

#[test]
fn a_plan_with_shares_for_two_keys_of_one_seed_does_not_open_with_the_keys_swapped() {
    use faraday_core::glance::Backup;
    use faraday_core::testkit;
    let c = testkit::compute();
    let (p, q) = ("m/48'/1'/0'/2'", "m/48'/1'/1'/2'");
    let (a, b) = (testkit::key(0, p), testkit::key(0, q));
    let (mut app, _) = shares_plan_kept(&[0], &a, &b, &[]);
    let holds = |app: &Faraday, w: usize| -> Vec<String> {
        match faraday_core::glance::of_now(&testkit::compute(), app, w)
            .unwrap()
            .backup
        {
            Backup::Plan { nodes, .. } => nodes
                .into_iter()
                .flat_map(|n| n.holds)
                .map(|h| h.to_string())
                .collect(),
            _ => Vec::new(),
        }
    };
    let w = app.session.wallets.len() - 1;
    assert!(
        holds(&app, w).iter().any(|h| h.contains("hare")),
        "{:?}",
        holds(&app, w)
    );
    app.session.wallets.clear();
    let text = format!("wsh(sortedmulti(2,{b}/<0;1>/*,{a}/<0;1>/*))");
    let w = app
        .session
        .add_wallet(
            &c,
            "Pair",
            &text,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    assert!(
        holds(&app, w).iter().all(|h| !h.contains("hare")),
        "{:?}",
        holds(&app, w)
    );
}

/// The plan records of vault `id`: each one's wallet, answers and notes.
type KeptPlan = (Vec<u8>, Vec<u8>, Vec<Vec<u8>>);

fn kept_plans(app: &Faraday, id: faraday_keys::vault::VaultId) -> Vec<KeptPlan> {
    use crate::common::RecordsOf;
    use faraday_vault::records::{field, kind};
    let all = app.vaults().store().contents_for_tests(id);
    all.of(kind::PLAN)
        .map(|(_, r)| {
            let r = faraday_keys::vault::RecordView::of(r);
            (
                r.public_text(field::PLAN_WALLET)
                    .unwrap_or_default()
                    .as_bytes()
                    .to_vec(),
                r.public_text(field::PLAN_ANSWERS)
                    .unwrap_or_default()
                    .as_bytes()
                    .to_vec(),
                r.public_all(field::PLAN_NOTE).map(<[u8]>::to_vec).collect(),
            )
        })
        .collect()
}

/// Loads the pair's wallet with its keys `y` then `x`, opens its backup and
/// saves the Standard plan; returns the toast the save left.
fn save_swapped_standard(app: &mut Faraday, x: &str, y: &str) -> Option<String> {
    use faraday_core::Action;
    use faraday_core::testkit;
    let c = testkit::compute();
    app.session.wallets.clear();
    let text = format!("wsh(sortedmulti(2,{y}/<0;1>/*,{x}/<0;1>/*))");
    let w = app
        .session
        .add_wallet(
            &c,
            "Pair",
            &text,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.press(Action::Backup(w));
    let shape = app.plan_shape_now(w);
    testkit::set_backup_answers(app, Answers::preset(&shape, Preset::Standard));
    testkit::backup_plan_save(app);
    app.toast_text().map(str::to_string)
}

#[test]
fn saving_the_backup_flow_with_the_keys_swapped_is_refused_over_a_shares_plan_of_the_other_order() {
    use faraday_core::plan::Mark;
    use faraday_core::testkit;
    let path = "m/48'/1'/0'/2'";
    let (x, y) = (testkit::key(0, path), testkit::key(1, path));
    let notes = [
        Note::Marked(At::Place(0), What::Share(0), Mark::Lost),
        Note::Checked(
            At::Place(1),
            What::Share(1),
            faraday_text::Text::of("2026-10-10"),
        ),
    ];
    let (mut app, id) = shares_plan_kept(&[0, 1], &x, &y, &notes);
    let before = kept_plans(&app, id);
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].2.len(), 2);
    let toast = save_swapped_standard(&mut app, &x, &y);
    assert_eq!(kept_plans(&app, id), before);
    assert_eq!(
        toast.as_deref(),
        Some(
            "The shares plan kept for this wallet has its keys in another order. \
             Load the wallet with its keys in that order to change the plan."
        )
    );
}

#[test]
fn saving_the_backup_flow_with_the_keys_swapped_replaces_a_standard_plan_and_keeps_its_notes() {
    use faraday_core::testkit;
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let path = "m/48'/1'/0'/2'";
    let (x, y) = (testkit::key(0, path), testkit::key(1, path));
    let (mut app, id) = shares_plan_kept(&[0, 1], &x, &y, &[]);
    // Replace the shares plan with a Standard one kept for the first order.
    let policy = app.session.wallets[0].policy().clone();
    let shape = app.plan_shape_now(0);
    let answers = Answers::preset(&shape, Preset::Standard);
    let note = Note::Checked(
        At::Place(0),
        What::Words(0),
        faraday_text::Text::of("2026-10-10"),
    );
    let record = Record::new(kind::PLAN)
        .with(field::PLAN_WALLET, policy.to_descriptor().as_bytes())
        .with(field::PLAN_ANSWERS, answers.to_text().as_bytes())
        .with(field::PLAN_NOTE, note.to_text().as_bytes());
    testkit::vault_edit(&mut app, id, |s| {
        s.remove_where(id, |r| r.public_text(field::PLAN_WALLET).is_some());
        s.insert(id, record);
    });
    let before = kept_plans(&app, id);
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].2.len(), 1);
    let toast = save_swapped_standard(&mut app, &x, &y);
    assert_ne!(
        toast.as_deref().map(|t| t.contains("another order")),
        Some(true)
    );
    let after = kept_plans(&app, id);
    assert_eq!(after.len(), 1, "the one record is replaced");
    assert_eq!(after[0].2.len(), 1, "its note is kept");
}
