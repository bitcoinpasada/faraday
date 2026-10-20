//! A layer's instructions, by hand (`docs/BACKUP-FLOW.md` §20.5, L4), and
//! the board edited by hand against a route's kinds (§20.13, L2b choice
//! 11). The instructions are a sheet to copy out: the places in order,
//! what to take at each, the passphrases needed by name, what to restore
//! with; never a secret's value, never kept or written anywhere. A route
//! whose kinds are set warns, under the board, where a place of it no
//! longer holds what they say.

use faraday_core::board::{BoardPress as BP, Chip, Thing, View, Zone};
use faraday_core::layers::LayersPress;
use faraday_core::plan::{At, What, map};
use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent, cstep, qstep};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

/// A device `height` tall, so a sheet's whole template is drawn at once.
fn device(height: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(180);
    let _ = app.frame();
    app
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn settle(app: &mut Faraday, until: impl Fn(&Faraday) -> bool) {
    for t in 1..60u64 {
        if until(app) {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

const PHRASE: &str = "test phrase";

/// A vault made and unlocked, so places' names are typed.
fn made_and_unlocked(app: &mut Faraday) {
    for i in 0..8u8 {
        app.event(Event::Entropy(osk_shell_api::EntropyBytes::new(
            [0x50 + i; 32],
        )));
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(app, PHRASE);
    }
    app.press(Action::Vault(V::CGo));
    settle(app, |a| a.screen == Screen::Unlock);
    type_text(app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults.open.is_empty());
    assert_eq!(app.vaults.open.len(), 1, "the vault did not unlock");
}

/// Back up of a 2 of 3 made in Create over the three test keys typed in
/// here, on the Paper and vault preset, with the two layers of
/// `tests/access_layers.rs`: Access layer 1 by a paper route, any 2 of
/// Places 1, 2 and 3, and by Place 1 alone knowing Vault 1's passphrase;
/// Access layer 2 by a paper route, any 2 of Places 2 and 3. `kinds`, Access
/// layer 2's route says what each place holds (one seed in words and a
/// part of the description, as kinds start). Then Work it out.
fn two_layers_worked(app: &mut Faraday, kinds: bool) {
    for n in 0..3 {
        app.press(Action::Entry(None));
        type_text(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        app.press(Action::EntryAdd);
    }
    testkit::create_knowing(app);
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .unwrap() as u8;
    app.press(Action::CKind(multi));
    let fps: Vec<[u8; 4]> = app
        .session
        .keys
        .iter()
        .rev()
        .take(3)
        .rev()
        .map(|k| k.fingerprint().0)
        .collect();
    for (slot, fp) in fps.into_iter().enumerate() {
        app.press(Action::CSlotHere(slot as u8, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(0));
    app.press(Action::BPreset(1));
    app.press(Action::BQ(qstep::LAYERS));
    let lp = |app: &mut Faraday, p: LayersPress| app.press(Action::Layers(p));
    lp(app, LayersPress::AddRoute(0));
    lp(app, LayersPress::Place(0, 1, 1));
    lp(app, LayersPress::Place(0, 1, 2));
    lp(app, LayersPress::Password(0, 1, 0));
    lp(app, LayersPress::AddLayer);
    lp(app, LayersPress::Place(1, 0, 0));
    if kinds {
        lp(app, LayersPress::KindsSet(1, 0));
    }
    lp(app, LayersPress::WorkItOut);
    assert_eq!(
        app.backup.as_ref().unwrap().q,
        Some(qstep::BOARD),
        "the board opens on the result"
    );
}

/// What the board's plan keeps on paper at place `p`, as the instructions
/// name it: each seed's words by fingerprint, each share by number.
fn paper_at(app: &Faraday, p: usize) -> Vec<String> {
    let (shape, a) = app.board_plan().expect("a plan");
    map(&shape, &a)
        .iter()
        .filter(|s| s.at == At::Place(p))
        .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
        .filter_map(|w| match w {
            What::Words(i) => Some(format!("Seed {}'s words", shape.seeds[i].name)),
            What::Share(j) => Some(format!("Share {} of the description", j + 1)),
            _ => None,
        })
        .collect()
}

/// The texts the sheet draws: from its title, the last `title` drawn
/// before its first "Go to", on.
fn sheet_texts(app: &mut Faraday, title: &str) -> Vec<String> {
    app.settle();
    let texts = app.drawn_texts();
    let go = texts
        .iter()
        .position(|t| t == "Go to")
        .expect("the sheet's first route");
    let start = texts[..go]
        .iter()
        .rposition(|t| t == title)
        .expect("the sheet's title");
    texts[start..].to_vec()
}

/// Whether `want` are drawn in this order in `texts`, others between.
fn in_order(texts: &[String], want: &[String]) -> bool {
    let mut it = texts.iter();
    want.iter().all(|w| it.any(|t| t == w))
}

fn s(x: &str) -> String {
    x.to_string()
}

/// Every place's take, in order: its number and name, then what it keeps
/// on paper (and `more` after it).
fn steps(app: &Faraday, places: &[(usize, &str)], more: &[(usize, &str)]) -> Vec<String> {
    let mut out = Vec::new();
    for (k, &(p, name)) in places.iter().enumerate() {
        out.push(format!("{}. {name}", k + 1));
        let paper = paper_at(app, p);
        // By hand: Work it out keeps one seed's words and one share at
        // each place (the 2-of-3 preset's result, `tests/access_layers.rs`).
        assert_eq!(paper.len(), 2, "Place {}: {paper:?}", p + 1);
        out.extend(paper);
        out.extend(more.iter().filter(|(q, _)| *q == p).map(|(_, m)| s(m)));
    }
    out
}

/// Each layer's instructions for the two layers worked out, by hand.
///
/// Access layer 1 has two routes, so each has its head. Route 1, its paper
/// route: any 2 of Places 1, 2 and 3; at each, from paper alone, that
/// place's seed's words and its share; no passphrase needed. Route 2:
/// Place 1 alone, any 1 of 1; there its words, its share and Vault 1's
/// stick (the vault holds every seed and the description); Vault 1's
/// passphrase needed, by name, and Vault 1 to open. Access layer 2 has one
/// route, so no head: any 2 of Places 2 and 3, each with its words and
/// share, no passphrase. Every route restores with Faraday's Restore, and
/// the plan names no watch-only software. No seed word is drawn: the
/// three test seeds are "bacon", "zebra" and "summer" twenty-four times.
#[test]
fn each_layers_instructions_list_its_places_in_order_what_to_take_and_what_it_needs() {
    let mut app = device(2400);
    two_layers_worked(&mut app, false);
    let text = app.backup.as_ref().unwrap().answers.to_text();

    app.press(Action::Board(BP::View(View::Layer(0))));
    let _ = app.frame();
    assert!(app.offers(Action::Instructions(0)), "the view offers them");
    app.press(Action::Instructions(0));
    assert_eq!(app.sheet, Some(Sheet::Instructions(0)));
    let texts = sheet_texts(&mut app, "Access layer 1");
    let places = [(0, "Place 1"), (1, "Place 2"), (2, "Place 3")];
    let mut want = vec![s("Access layer 1"), s("Route 1 · paper"), s("Go to")];
    want.push(s("Any 2 of 3"));
    want.extend(steps(&app, &places, &[]));
    want.extend([s("Passphrases needed"), s("None")]);
    want.extend([s("Restore with"), s("Faraday's Restore")]);
    want.extend([s("Route 2"), s("Go to"), s("Any 1 of 1")]);
    want.extend(steps(&app, &places[..1], &[(0, "Vault 1's stick")]));
    want.extend([s("Passphrases needed"), s("Vault 1's passphrase")]);
    want.extend([s("Open"), s("Vault 1")]);
    want.extend([s("Restore with"), s("Faraday's Restore")]);
    assert!(in_order(&texts, &want), "{texts:#?}\nwants {want:#?}");
    // Its paper route takes no stick.
    let route2 = texts.iter().position(|t| t == "Route 2").unwrap();
    assert!(!texts[..route2].iter().any(|t| t.contains("stick")));
    assert!(!texts.iter().any(|t| t == "Watch with"));
    // The places it takes from are exactly its own: three, then one.
    let numbered = texts
        .iter()
        .filter(|t| t.len() > 3 && t.as_bytes()[1] == b'.' && t.as_bytes()[2] == b' ')
        .count();
    assert_eq!(numbered, 4);
    for word in ["bacon", "zebra", "summer"] {
        assert!(
            !texts.iter().any(|t| t.to_lowercase().contains(word)),
            "a seed's word is drawn"
        );
    }

    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    app.press(Action::Board(BP::View(View::Layer(1))));
    app.press(Action::Instructions(1));
    let texts = sheet_texts(&mut app, "Access layer 2");
    let mut want = vec![s("Access layer 2"), s("Go to"), s("Any 2 of 2")];
    want.extend(steps(&app, &places[1..], &[]));
    want.extend([s("Passphrases needed"), s("None")]);
    want.extend([s("Restore with"), s("Faraday's Restore")]);
    assert!(in_order(&texts, &want), "{texts:#?}\nwants {want:#?}");
    assert!(!texts.iter().any(|t| t.starts_with("Route ")), "one route");
    assert!(!texts.iter().any(|t| t == "1. Place 1"), "not its place");
    app.press(Action::Cancel);
    assert_eq!(app.backup.as_ref().unwrap().answers.to_text(), text);
}

/// The Access layers card's layer heading offers each layer's
/// instructions too, the same sheet.
#[test]
fn the_access_layers_card_opens_a_layers_instructions_from_its_heading() {
    let mut app = device(2400);
    two_layers_worked(&mut app, false);
    app.press(Action::BQ(qstep::LAYERS));
    let _ = app.frame();
    assert!(app.offers(Action::Instructions(1)));
    app.press(Action::Instructions(1));
    let texts = sheet_texts(&mut app, "Access layer 2");
    assert!(in_order(
        &texts,
        &[s("Go to"), s("Any 2 of 2"), s("1. Place 2")]
    ));
}

/// Every storage command the app has asked, drained.
fn drained(app: &mut Faraday) -> Vec<StorageCommand> {
    let mut out = Vec::new();
    while let Some(c) = app.poll_storage() {
        out.push(c);
    }
    out
}

/// What a lock keeps of the session: the files of its save.
fn kept_on_lock(app: &mut Faraday) -> Vec<(String, Vec<u8>)> {
    drained(app);
    app.press(Action::Lock);
    drained(app)
        .into_iter()
        .filter_map(|c| match c {
            StorageCommand::SaveBoxes { kept, .. } => Some(kept),
            _ => None,
        })
        .next_back()
        .expect("a lock saves what it keeps")
}

/// Opening a layer's instructions, drawing them and leaving leaves
/// nothing: no storage command is asked (no file written to any stick,
/// nothing saved), the plan's text is as it was, and what a lock keeps is
/// byte for byte what the same session keeps without the sheet.
#[test]
fn leaving_the_instructions_leaves_the_plan_the_kept_state_and_the_sticks_unchanged() {
    let mut plain = device(2400);
    two_layers_worked(&mut plain, false);
    let mut seen = device(2400);
    two_layers_worked(&mut seen, false);
    let text = seen.backup.as_ref().unwrap().answers.to_text();
    drained(&mut seen);
    for l in 0..2u8 {
        seen.press(Action::Board(BP::View(View::Layer(usize::from(l)))));
        plain.press(Action::Board(BP::View(View::Layer(usize::from(l)))));
        seen.press(Action::Instructions(l));
        for _ in 0..3 {
            seen.settle();
            let _ = seen.frame();
        }
        seen.press(Action::Cancel);
        let _ = seen.frame();
    }
    let asked = drained(&mut seen);
    assert!(asked.is_empty(), "{} storage commands asked", asked.len());
    assert_eq!(seen.backup.as_ref().unwrap().answers.to_text(), text);
    assert_eq!(kept_on_lock(&mut seen), kept_on_lock(&mut plain));
}

/// With a vault open and Places 1 and 3 renamed, the instructions name the
/// places so; the vault's passphrase is named, never drawn.
#[test]
fn with_a_vault_open_the_instructions_use_the_places_names() {
    const KITCHEN: &str = "Kitchen drawer";
    const BANK: &str = "Bank box";
    let mut app = device(2400);
    made_and_unlocked(&mut app);
    two_layers_worked(&mut app, false);
    for (p, name) in [(0u8, KITCHEN), (2, BANK)] {
        app.press(Action::BName(p));
        type_text(&mut app, name);
        app.event(Event::Key(Key::Enter));
        assert_eq!(app.place_name(usize::from(p)), name);
    }
    app.press(Action::Board(BP::View(View::Layer(0))));
    app.press(Action::Instructions(0));
    let texts = sheet_texts(&mut app, "Access layer 1");
    let want = [
        s("Route 1 · paper"),
        format!("1. {KITCHEN}"),
        s("2. Place 2"),
        format!("3. {BANK}"),
        s("Route 2"),
        format!("1. {KITCHEN}"),
        s("Vault 1's stick"),
        s("Vault 1's passphrase"),
    ];
    assert!(in_order(&texts, &want), "{texts:#?}");
    assert!(
        !texts.iter().any(|t| t.contains(PHRASE)),
        "a passphrase drawn"
    );
}

/// Every text drawn down the page, a frame at a time from its top.
fn frames_down(app: &mut Faraday) -> Vec<String> {
    app.event(Event::Scroll {
        x: 600,
        y: 400,
        dy: -20_000,
    });
    let mut out = Vec::new();
    for _ in 0..30 {
        app.settle();
        out.extend(app.drawn_texts());
        app.event(Event::Scroll {
            x: 600,
            y: 400,
            dy: 150,
        });
    }
    out
}

const WARNING: &str = "Access layer 2, route 1: Place 3 holds no seed in words";

/// Access layer 2's route says one seed in words and a part of the
/// description at each of Places 2 and 3; Work it out keeps a seed's words
/// and a share at each, so nothing is warned. The seed's words taken off
/// Place 3 by hand leave it no seed in words: a warning line names the
/// route, the place and the kind, and the board takes the change (a
/// warning, never a refusal). Put back, the line goes.
#[test]
fn a_seed_taken_off_a_routes_place_warns_by_its_kinds_and_put_back_does_not() {
    let mut app = device(768);
    two_layers_worked(&mut app, true);
    let lines = frames_down(&mut app);
    assert!(
        !lines
            .iter()
            .any(|t| t.starts_with("Access layer 2, route 1:")),
        "nothing warned after Work it out"
    );
    let (shape, a) = app.board_plan().unwrap();
    let seed = map(&shape, &a)
        .iter()
        .filter(|s| s.at == At::Place(2))
        .flat_map(|s| s.holds.iter())
        .find_map(|(w, _)| match w {
            What::Words(i) => Some(*i),
            _ => None,
        })
        .expect("Place 3 keeps a seed's words");
    app.press(Action::Board(BP::Off(Zone::Paper(2), Thing::Seed(seed))));
    let (shape, a) = app.board_plan().unwrap();
    assert!(
        !map(&shape, &a)
            .iter()
            .filter(|s| s.at == At::Place(2))
            .any(|s| s.holds.iter().any(|(w, _)| *w == What::Words(seed))),
        "the change is taken"
    );
    assert!(frames_down(&mut app).iter().any(|t| t == WARNING));

    app.press(Action::Board(BP::Chip(Chip::List(Thing::Seed(seed)))));
    app.press(Action::Board(BP::Zone(Zone::Paper(2))));
    assert!(!frames_down(&mut app).iter().any(|t| t == WARNING));
}
