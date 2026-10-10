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
        assert!(!a.split, "{preset:?}: the whole sheet in each");
        assert_eq!(a.seeds[seeds::VAULT], preset != Preset::Paper, "{preset:?}");
        let places: Vec<_> = map(&shape, &a)
            .into_iter()
            .filter(|s| matches!(s.at, At::Place(_)))
            .collect();
        assert_eq!(places.len(), 3);
        for (p, spot) in places.iter().enumerate() {
            let holds: Vec<What> = spot.holds.iter().map(|(w, _)| *w).collect();
            assert!(holds.contains(&What::Words(p)), "{preset:?}: its seed");
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
    assert!(!checklist(&shape, &a).iter().any(|i| match i {
        Item::Templates | Item::Copy(_) => true,
        Item::Vault(v) => !a.vault_seeds(&shape, *v).is_empty(),
        _ => false,
    }));
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
    let whole = Answers::preset(&shape, Preset::Paper);
    // The whole sheet in each place: one place found sees the balance.
    assert_eq!(check(&shape, &whole).balance, Found::Yes);
    let mut a = whole.clone();
    a.toggle(&shape, Question::Split, 1);
    assert!(a.split, "Its own share is the second row");
    let c = check(&shape, &a);
    assert_eq!(c.lost, Lost::Yes);
    // No one place holds two seeds, nor every key.
    assert_eq!(c.spend, Found::No);
    assert_eq!(c.balance, Found::No);
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
    // No paper seed: the seed is in the vault alone, its stick in place 2.
    a.toggle(&shape, Question::Seeds, seeds::WORDS);
    a.toggle(&shape, Question::Sticks(0), 0);
    let c = check(&shape, &a);
    assert_eq!(
        c.lost,
        Lost::WithVault,
        "the vault's sticks are in both places"
    );
    assert_eq!(c.spend, Found::OnlyWithVault);
    // One stick, in place 2: losing it loses the seed.
    a.toggle(&shape, Question::Sticks(0), 0);
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
    // Nothing by hand or into a vault, so the item is open to reach.
    app.press(Action::BAnswer(qrow::WALLET, wallet::VAULT as u8));
    app.press(Action::BAnswer(qrow::SEEDS, seeds::WORDS as u8));
    app.press(Action::BAnswer(qrow::SEEDS, seeds::VAULT as u8));
    app.press(Action::BChecklist);
    assert!(
        app.backup_items().contains(&bstep::PUBLIC),
        "the checklist has the public files item"
    );
    app.press(Action::BStep(bstep::PUBLIC));
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

// ---------------------------------------------------------------------
// `docs/NEW-WALLET.md` §4: the plan leads with the whole wallet sheet;
// shares come second, with a Learn page and a slider.
// ---------------------------------------------------------------------

/// An app on the test network with Savings (2-of-3) and a 3-of-5 loaded,
/// and the backup of wallet `name` open on its Places question with the
/// Paper preset.
fn places_of(name: &str) -> faraday_core::Faraday {
    use faraday_core::{Action, qstep, testkit};
    use osk_shell_api::App;
    let mut app = testkit::started();
    app.session = testkit::session();
    app.session
        .add_wallet("Savings", &testkit::savings(), "test")
        .unwrap();
    app.session
        .add_wallet("Five", &testkit::three_of_five(), "test")
        .unwrap();
    let w = app
        .session
        .wallets
        .iter()
        .position(|w| w.name == name)
        .unwrap();
    app.press(Action::Backup(w));
    app.press(Action::BPreset(0));
    // With no seed here the preset opens Places itself.
    if app.backup.as_ref().unwrap().q != Some(qstep::PLACES) {
        app.press(Action::BQ(qstep::PLACES));
    }
    let _ = app.frame();
    app
}

/// A multisig's plan keeps the whole wallet sheet in each place until
/// the person ticks Its own share.
#[test]
fn a_fresh_multisig_plan_keeps_the_whole_sheet_in_each_place() {
    use faraday_core::{Action, qrow};
    let app = places_of("Savings");
    let b = app.backup.as_ref().unwrap();
    assert!(!b.answers.split);
    let shape = app.plan_shape(b.wallet);
    for p in 0..b.answers.places {
        assert!(b.answers.sheet_at(&shape, p), "place {p}: the whole sheet");
        assert!(b.answers.shares_at(&shape, p).is_empty(), "place {p}");
    }
    assert!(app.offers(Action::BAnswer(qrow::SPLIT, 1)), "Its own share");
}

/// Ticking Its own share on a 3-of-5 shows a slider of the keys left off
/// each share, from 0 to 2; the checklist's Shares card has the same.
#[test]
fn its_own_share_on_a_three_of_five_offers_0_to_2_keys_left_off() {
    use faraday_core::seeds::SLIDE_OMIT;
    use faraday_core::{Action, bstep, qrow};
    use osk_shell_api::App;
    let mut app = places_of("Five");
    assert!(!app.offers(Action::Slide(SLIDE_OMIT, 0)), "no slider yet");
    app.press(Action::BAnswer(qrow::SPLIT, 1));
    let _ = app.frame();
    for k in 0..=2 {
        assert!(
            app.offers(Action::Slide(SLIDE_OMIT, k)),
            "{k} keys left off"
        );
    }
    assert!(!app.offers(Action::Slide(SLIDE_OMIT, 3)), "no stop past 2");
    app.press(Action::Slide(SLIDE_OMIT, 1));
    assert_eq!(app.backup.as_ref().unwrap().answers.omit, 1);
    app.press(Action::BChecklist);
    app.press(Action::BStep(bstep::SHEETS));
    let _ = app.frame();
    assert!(app.offers(Action::Slide(SLIDE_OMIT, 2)), "the Shares card");
    app.press(Action::Slide(SLIDE_OMIT, 2));
    assert_eq!(app.backup.as_ref().unwrap().answers.omit, 2);
}

/// What is a share? opens Learn on the page about shares; closing it
/// leaves the plan on Places.
#[test]
fn what_is_a_share_opens_the_page_and_back_returns_to_places() {
    use faraday_core::{Action, Screen, Sheet, qstep};
    let mut app = places_of("Savings");
    assert!(app.offers(Action::LearnShares));
    app.press(Action::LearnShares);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert_eq!(
        app.learn.pages[app.learn.page].title,
        "Shares of a wallet description"
    );
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup.as_ref().unwrap().q, Some(qstep::PLACES));
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
    let boxes = map(shape, a);
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
    map(shape, a)
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
    let shape = two_here();
    let a = Answers::preset(&shape, Preset::PaperVault);
    let vaults: Vec<(At, Vec<What>)> = map(&shape, &a)
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
        map(&shape, &a)[2]
            .holds
            .iter()
            .find(|(w, _)| matches!(w, What::VaultStick(_)))
            .map(|(w, _)| w.label(&shape, faraday_core::Medium::Stick)),
        Some("Stick with Vault 1".to_string())
    );
    // So no one place found spends, even with a vault's passphrase; any
    // one lost leaves the rest to rebuild it.
    let c = check(&shape, &a);
    assert_eq!(c.spend, Found::No);
    assert_eq!(c.lost, Lost::Yes);
    assert_eq!(
        checklist(&shape, &a)
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
        m: 3,
        keys: 5,
        seeds: (1..=5)
            .map(|k| seed(&format!("aaaa000{k}"), k <= 3))
            .collect(),
        splits: true,
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
    // A single key: its words at Place 1, the vault's stick at Place 2.
    let shape = one_key();
    let a = Answers::preset(&shape, Preset::PaperVault);
    assert_eq!(a.words_at(&shape, 0), vec![0]);
    assert_eq!(sticks_at(&shape, &a, 0), Vec::<usize>::new());
    assert_eq!(sticks_at(&shape, &a, 1), vec![0]);
}

#[test]
fn a_plan_saved_with_one_vault_still_loads() {
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
    let vault = map(&shape, &a)
        .into_iter()
        .find(|s| s.at == At::Vault(0))
        .unwrap();
    assert_eq!(vault.holds.len(), 4, "three seeds and the wallet");
}

/// A single key whose words are unticked keeps its only copy in Vault 1,
/// and Vault 1's stick still has a place (`docs/NEW-WALLET.md` §14.2);
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
    let shape = one_key();
    let mut a = Answers::preset(&shape, Preset::PaperVault);
    a.toggle(&shape, Question::Seeds, seeds::WORDS);
    let stick = (0..a.places).find(|&p| a.stick_at(0, p)).unwrap();
    assert_eq!(check(&shape, &a).lost, Lost::No);
    let name = |p: usize| format!("Place {}", p + 1);
    assert_eq!(
        faraday_core::plan::lost_why(&shape, &a, &name, "stick").as_deref(),
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
        faraday_core::plan::lost_why(&shape, &a, &name, "stick"),
        None
    );
}

/// A vault left with no stick place holds Places: its Continue gives way
/// to a line that says so, and the checklist is not made.
#[test]
fn a_vault_with_no_stick_place_holds_places() {
    use faraday_core::{Action, BStage, qrow, qstep};
    use osk_shell_api::App;
    let mut app = places_of("Savings");
    let w = app.backup.as_ref().unwrap().wallet;
    let shape = app.plan_shape(w);
    // The description into a vault: Vault 1, its stick given a place.
    app.press(Action::BAnswer(qrow::WALLET, wallet::VAULT as u8));
    let a = app.backup.as_ref().unwrap().answers.clone();
    assert_eq!(a.vaults_made(&shape), vec![0]);
    let at = (0..a.places).find(|&p| a.stick_at(0, p)).expect("a place");
    // Its stick unticked.
    app.press(Action::BAnswer(qrow::STICKS, at as u8));
    let _ = app.frame();
    assert!(!app.offers(Action::BQNext(qstep::PLACES)));
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t.contains("Vault 1's stick needs a place")),
        "{:?}",
        app.drawn_texts()
    );
    app.press(Action::BQNext(qstep::PLACES));
    assert_eq!(app.backup.as_ref().unwrap().q, Some(qstep::PLACES));
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Plan);
    // Ticked again, the way on is back.
    app.press(Action::BAnswer(qrow::STICKS, at as u8));
    app.press(Action::BQNext(qstep::PLACES));
    assert_ne!(app.backup.as_ref().unwrap().q, Some(qstep::PLACES));
}
