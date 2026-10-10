//! The backup's plan on its own: each preset fills the answers the
//! wallet's shape calls for, the check says what one place lost or found
//! gives, the checklist lists only what the plan needs, and the answers
//! read back as the vault keeps them.

use faraday_core::plan::{
    Answers, At, Found, Item, Lost, Preset, Question, Seed, Shape, What, check, checklist, form,
    map, seeds, software, wallet,
};

fn seed(name: &str, here: bool) -> Seed {
    Seed {
        name: name.to_string(),
        here,
        passphrase: false,
    }
}

fn one_key() -> Shape {
    Shape {
        m: 1,
        keys: 1,
        seeds: vec![seed("aaaa0001", true)],
        splits: false,
    }
}

fn two_of_three() -> Shape {
    Shape {
        m: 2,
        keys: 3,
        seeds: vec![
            seed("aaaa0001", true),
            seed("aaaa0002", true),
            seed("aaaa0003", true),
        ],
        splits: true,
    }
}

#[test]
fn each_preset_fills_a_one_key_wallets_answers() {
    let shape = one_key();
    for preset in Preset::ALL {
        let a = Answers::preset(&shape, preset);
        let vault = preset != Preset::Paper;
        assert!(a.seeds[seeds::WORDS], "{preset:?}: paper words");
        assert!(
            !a.seeds[seeds::SEEDQR] && !a.seeds[seeds::FILE],
            "{preset:?}"
        );
        assert_eq!(a.seeds[seeds::VAULT], vault, "{preset:?}");
        assert_eq!(a.places, 2, "{preset:?}: two places");
        assert!(a.wallet[wallet::PAPER], "{preset:?}: the sheet in each");
        assert_eq!(a.wallet[wallet::VAULT], vault, "{preset:?}");
        assert_eq!(
            a.wallet[wallet::SOFTWARE],
            preset == Preset::PaperVaultSoftware,
            "{preset:?}"
        );
        assert!(a.form[form::QR] && !a.form[form::TEXT], "{preset:?}: as QR");
        // The sheet in each place, the words in each.
        for spot in map(&shape, &a)
            .iter()
            .filter(|s| matches!(s.at, At::Place(_)))
        {
            let holds: Vec<What> = spot.holds.iter().map(|(w, _)| *w).collect();
            assert!(holds.contains(&What::Sheet), "{preset:?} {spot:?}");
            assert!(holds.contains(&What::Words(0)), "{preset:?} {spot:?}");
        }
    }
}

#[test]
fn each_preset_fills_a_two_of_threes_answers() {
    let shape = two_of_three();
    for preset in Preset::ALL {
        let a = Answers::preset(&shape, preset);
        assert_eq!(a.places, 3, "{preset:?}: a place per key");
        assert!(a.split, "{preset:?}: a share each");
        assert_eq!(a.seeds[seeds::VAULT], preset != Preset::Paper, "{preset:?}");
        let places: Vec<_> = map(&shape, &a)
            .into_iter()
            .filter(|s| matches!(s.at, At::Place(_)))
            .collect();
        assert_eq!(places.len(), 3);
        for (p, spot) in places.iter().enumerate() {
            let holds: Vec<What> = spot.holds.iter().map(|(w, _)| *w).collect();
            assert!(holds.contains(&What::Words(p)), "{preset:?}: its seed");
            assert!(holds.contains(&What::Share(p)), "{preset:?}: its share");
            assert!(!holds.contains(&What::Sheet), "{preset:?}: no whole sheet");
        }
    }
}

#[test]
fn seeds_held_elsewhere_get_no_paper_here_and_watch_only_asks_nothing_of_seeds() {
    let mut shape = two_of_three();
    shape.seeds[1].here = false;
    shape.seeds[2].here = false;
    let a = Answers::preset(&shape, Preset::Paper);
    let boxes = map(&shape, &a);
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
    assert!(
        !checklist(&shape, &a)
            .iter()
            .any(|i| matches!(i, Item::Templates | Item::Copy(_) | Item::SeedsVault))
    );
}

#[test]
fn one_place_with_paper_only_does_not_survive_losing_it() {
    let shape = one_key();
    let mut a = Answers::preset(&shape, Preset::Paper);
    a.toggle(&shape, Question::Places, 1);
    assert_eq!(a.places, 1);
    let c = check(&shape, &a);
    assert_eq!(c.lost, Lost::No);
    assert_eq!(c.spend, Found::Yes);
}

#[test]
fn three_places_with_split_shares_survive_any_one_lost() {
    let shape = two_of_three();
    let a = Answers::preset(&shape, Preset::Paper);
    let c = check(&shape, &a);
    assert_eq!(c.lost, Lost::Yes);
    // No one place holds two seeds, nor every key.
    assert_eq!(c.spend, Found::No);
    assert_eq!(c.balance, Found::No);
    // The whole sheet in each place: one place found sees the balance.
    let mut whole = a.clone();
    whole.toggle(&shape, Question::Split, 1);
    assert_eq!(check(&shape, &whole).balance, Found::Yes);
}

#[test]
fn a_place_with_a_quorum_of_seeds_on_paper_can_spend() {
    let shape = two_of_three();
    let mut a = Answers::preset(&shape, Preset::Paper);
    a.toggle(&shape, Question::Places, 2);
    // Place 1 keeps seeds 1 and 3, with shares 1 and 3, which hold every
    // key.
    assert_eq!(a.words_at(&shape, 0), vec![0, 2]);
    assert_eq!(check(&shape, &a).spend, Found::Yes);
}

#[test]
fn the_vault_counts_with_its_passphrase() {
    let shape = one_key();
    let mut a = Answers::preset(&shape, Preset::PaperVault);
    // No paper seed: the seed is in the vault alone, its stick in place 1.
    a.toggle(&shape, Question::Seeds, seeds::WORDS);
    a.toggle(&shape, Question::Sticks, 1);
    let c = check(&shape, &a);
    assert_eq!(
        c.lost,
        Lost::WithVault,
        "the vault's sticks are in both places"
    );
    assert_eq!(c.spend, Found::OnlyWithVault);
    // One stick, in place 1: losing it loses the seed.
    a.toggle(&shape, Question::Sticks, 1);
    assert_eq!(check(&shape, &a).lost, Lost::No);
}

#[test]
fn a_passphrase_never_shares_a_place_with_its_words() {
    let mut shape = two_of_three();
    shape.seeds[0].passphrase = true;
    let mut a = Answers::preset(&shape, Preset::Paper);
    assert!(!a.pass_at(0, 0), "not with its words");
    assert!(a.pass_at(0, 1), "the next place keeps it");
    a.toggle(&shape, Question::Passphrase(0), 0);
    assert!(!a.pass_at(0, 0), "a tick beside its words is refused");
    // Its words without it rebuild nothing: losing place 2 leaves seed 1
    // without its passphrase, and seed 3 alone.
    assert_eq!(check(&shape, &a).lost, Lost::No);
}

#[test]
fn the_checklist_lists_only_what_the_plan_needs() {
    let shape = two_of_three();
    let paper = checklist(&shape, &Answers::preset(&shape, Preset::Paper));
    assert_eq!(
        paper,
        vec![
            Item::Templates,
            Item::Copy(0),
            Item::Copy(1),
            Item::Copy(2),
            Item::Sheets,
            Item::Envelopes
        ]
    );
    let mut a = Answers::preset(&shape, Preset::PaperVaultSoftware);
    let all = checklist(&shape, &a);
    for item in [Item::SeedsVault, Item::WalletVault, Item::ShowDescriptor] {
        assert!(all.contains(&item), "{item:?}");
    }
    assert!(
        !all.contains(&Item::PublicFiles),
        "QR only: nothing on a stick"
    );
    a.toggle(&shape, Question::Form, form::TEXT);
    a.toggle(&shape, Question::Software, software::SPARROW);
    assert!(checklist(&shape, &a).contains(&Item::PublicFiles));
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

/// A 2-of-3 made in Create over the three test keys, its backup opened
/// from Create's Back up card with every software chosen and the
/// description going as files: the public
/// files item offers each file Create's Public files card offered, each
/// named for the wallet.
#[test]
fn the_public_files_item_offers_the_files_create_offered_for_the_wallet() {
    use faraday_core::create::NewKind;
    use faraday_core::{Action, bstep, cstep, qrow, testkit};
    use osk_shell_api::{App, Event, Key};
    let mut app = testkit::started();
    for word in ["bacon", "zebra", "summer"] {
        app.press(Action::Entry(None));
        for c in testkit::test_words(word).chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        app.press(Action::EntryAdd);
    }
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi));
    for slot in 0..3u8 {
        let fp = app.session.keys[usize::from(slot)].master.fingerprint().0;
        app.press(Action::CSlotHere(slot, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let built = app.create.as_ref().and_then(|c| c.built).expect("made");
    app.press(Action::CNext(cstep::CHECK));
    let k = Preset::ALL
        .iter()
        .position(|p| *p == Preset::PaperVaultSoftware)
        .unwrap() as u8;
    app.press(Action::CBackup(k));
    for row in 0..5u8 {
        if !app.backup.as_ref().unwrap().answers.software[usize::from(row)] {
            app.press(Action::BAnswer(qrow::SOFTWARE, row));
        }
    }
    // The description goes as files too: the public files item.
    if !app.backup.as_ref().unwrap().answers.wallet[wallet::FILES] {
        app.press(Action::BAnswer(qrow::WALLET, wallet::FILES as u8));
    }
    app.press(Action::BChecklist);
    assert!(
        app.backup_items().contains(&bstep::PUBLIC),
        "the checklist has the public files item"
    );
    app.press(Action::BStep(bstep::PUBLIC));
    let _ = app.frame();
    assert!(
        app.offers(Action::PublicOut(built, 1)),
        "the item is open on the descriptor"
    );
    let stem = faraday_core::file_stem(&app.session.wallets[built].name);
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
            app.outbox.iter().any(|f| f.name == name),
            "{name} reached the Outbox"
        );
    }
}

/// A single-key wallet made in Create from a key whose New key was given
/// a passphrase: its plan asks where the passphrase goes
/// (`docs/NEW-WALLET.md` §3.5).
#[test]
fn a_single_key_create_with_a_passphrase_asks_where_the_passphrase_goes() {
    use faraday_core::keygen::Way;
    use faraday_core::{Action, cstep, qstep};
    use osk_shell_api::{App, Event, Key};
    let mut app = faraday_core::testkit::started();
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
        for c in "Ride the 7 bus".chars() {
            app.event(Event::Key(Key::Char(c)));
        }
    }
    app.press(Action::KLock);
    app.press(Action::KNext);
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.open, Some(cstep::BACKUP));
    app.press(Action::CBackup(0));
    assert!(app.backup.is_some(), "the backup is open");
    assert!(
        app.backup_questions().contains(&qstep::PASSPHRASE),
        "the plan asks where the passphrase goes"
    );
}
