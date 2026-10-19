//! Access layers (`docs/BACKUP-FLOW.md` §20, F38): what a backup lets
//! each layer do by each of its routes, what a stranger who finds places
//! gets, and the layers kept with the plan: their routes in the plan's
//! text, their names in the vault alone.

use faraday_core::access::{
    Layer, Reach, Route, balance_margin, every_layer_on_paper, kept, layer_met, layer_name,
    layer_names, push_layer_names, route_met, stranger_margin,
};
use faraday_core::plan::{Answers, At, Preset, Seed, Shape, Spot, What, map, stored_map, tag_of};
use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent, qstep};
use faraday_vault::records::{Contents, Record, field, kind};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn seed(name: &str) -> Seed {
    Seed {
        name: name.to_string(),
        here: true,
        passphrase: false,
    }
}

fn shape(m: usize, n: usize) -> Shape {
    Shape {
        m,
        keys: n,
        seeds: (0..n).map(|i| seed(&format!("aaaa000{i}"))).collect(),
        splits: n > 1,
        vaults: 0,
    }
}

fn route(places: &[usize], any: usize, passwords: &[usize]) -> Route {
    Route {
        places: places.to_vec(),
        any,
        passwords: passwords.to_vec(),
        passphrases: Vec::new(),
        paper: false,
        written: Vec::new(),
    }
}

/// A paper route: any `any` of `places`, from their paper alone.
fn paper(places: &[usize], any: usize) -> Route {
    Route {
        paper: true,
        ..route(places, any, &[])
    }
}

fn layer(routes: Vec<Route>) -> Layer {
    Layer { routes }
}

/// A plan whose map is `places` (what each place keeps on paper or on
/// its sticks) and `vaults` (what each vault holds).
fn drawn(shape: &Shape, places: &[&[What]], vaults: &[&[What]]) -> Answers {
    let spot = |at: At, holds: &[What]| Spot {
        at,
        holds: holds.iter().map(|&w| (w, tag_of(at, w))).collect(),
    };
    let mut m: Vec<Spot> = places
        .iter()
        .enumerate()
        .map(|(p, h)| spot(At::Place(p), h))
        .collect();
    m.extend(
        vaults
            .iter()
            .enumerate()
            .map(|(v, h)| spot(At::Vault(v), h)),
    );
    let mut a = Answers::empty(shape);
    a.set_map(m);
    a
}

/// The 2-of-3 Paper and vault preset keeps, at each place, its seed's
/// words, the wallet sheet and the stick of the vault holding that same
/// seed with the description. Worked by hand: one place gives one seed
/// however its vault is opened, so Access layer 1 at Place 1 knowing
/// Vault 1's password cannot spend; any two places on paper can. A
/// stranger at one place cannot spend (margin 1) but sees the balance
/// there (margin 0).
#[test]
fn paper_and_vault_on_a_two_of_three_spends_from_any_two_places_and_not_from_one() {
    let s = shape(2, 3);
    let a = Answers::preset(&s, Preset::PaperVault);
    assert!(!route_met(&s, &a, &route(&[0], 1, &[0])));
    assert!(route_met(&s, &a, &route(&[0, 1, 2], 2, &[])));
    assert!(!route_met(&s, &a, &route(&[0, 1, 2], 1, &[0, 1, 2])));
    assert_eq!(stranger_margin(&s, &a), 1);
    assert_eq!(balance_margin(&s, &a), 0);
}

/// The same preset with Vault 2's stick kept at Place 1 too: Access layer
/// 1 knowing Vault 2's password spends from Place 1 alone (Seed 1's
/// words, Seed 2 in the vault, the sheet). A stranger there, without the
/// password, has one seed: the margin stays 1.
#[test]
fn a_second_vaults_stick_and_its_password_let_one_place_spend() {
    let s = shape(2, 3);
    let mut a = Answers::preset(&s, Preset::PaperVault);
    a.sticks[1][0] = true;
    let one = layer(vec![route(&[0], 1, &[1])]);
    assert!(layer_met(&s, &a, &one));
    assert!(!route_met(&s, &a, &route(&[0], 1, &[0])));
    assert_eq!(stranger_margin(&s, &a), 1);
}

/// A 3-of-5 on paper, a seed and the sheet at each place, is out of a
/// stranger's reach at any two places (margin 2). With Place 1 keeping
/// two seeds more, its three seeds and the sheet spend alone: margin 0.
/// Keeping only one seed more there, Place 1 and any other place spend:
/// margin 1.
#[test]
fn one_place_with_a_quorum_and_the_description_leaves_a_stranger_no_margin() {
    let s = shape(3, 5);
    let a = Answers::preset(&s, Preset::Paper);
    assert_eq!(stranger_margin(&s, &a), 2);
    assert_eq!(balance_margin(&s, &a), 0);

    let with = |extra: &[What]| {
        let mut m = stored_map(&s, &a);
        let place = m.iter_mut().find(|x| x.at == At::Place(0)).unwrap();
        for &w in extra {
            place.holds.push((w, tag_of(At::Place(0), w)));
        }
        let mut b = a.clone();
        b.set_map(m);
        b
    };
    let three = with(&[What::Words(1), What::Words(2)]);
    assert_eq!(stranger_margin(&s, &three), 0);
    assert!(route_met(&s, &three, &route(&[0], 1, &[])));
    let two = with(&[What::Words(1)]);
    assert_eq!(stranger_margin(&s, &two), 1);
}

/// A route "any 2 of Places 1, 2 and 3" on a 2-of-3 on paper is met: any
/// two of them spend. Asked of any one of them, it is not.
#[test]
fn any_two_of_three_places_spend_and_no_one_of_them_does() {
    let s = shape(2, 3);
    let a = Answers::preset(&s, Preset::Paper);
    assert!(route_met(&s, &a, &route(&[0, 1, 2], 2, &[])));
    assert!(!route_met(&s, &a, &route(&[0, 1, 2], 1, &[])));
    // Two named places, both needed.
    assert!(route_met(&s, &a, &route(&[0, 2], 2, &[])));
    // More places asked for than the route has: never met.
    assert!(!route_met(&s, &a, &route(&[0, 1], 3, &[])));
}

/// Place 1 keeps Seed 1's words and Vault 1's stick (the vault holds Seed
/// 2 and the description), Place 2 the sheet, Place 3 Seed 3 and the
/// sheet. A paper route through Places 1 and 2 has one seed; with Vault
/// 1's password written on paper at Place 2 it opens the vault and
/// spends. Knowing the password does the same without the paper.
#[test]
fn a_vault_password_written_at_a_place_opens_the_vault_for_a_route_through_it() {
    let s = shape(2, 3);
    let vault: &[What] = &[What::Seed(1), What::Wallet];
    let plain = drawn(
        &s,
        &[
            &[What::Words(0), What::VaultStick(0)],
            &[What::Sheet],
            &[What::Words(2), What::Sheet],
        ],
        &[vault],
    );
    let written = drawn(
        &s,
        &[
            &[What::Words(0), What::VaultStick(0)],
            &[What::Sheet, What::VaultPassword(0)],
            &[What::Words(2), What::Sheet],
        ],
        &[vault],
    );
    let paper = route(&[0, 1], 2, &[]);
    assert!(!route_met(&s, &plain, &paper));
    assert!(route_met(&s, &written, &paper));
    assert!(route_met(&s, &plain, &route(&[0, 1], 2, &[0])));
    // The password written at Place 2 counts only where Place 2 is found.
    assert!(!route_met(&s, &written, &route(&[0], 1, &[])));
    // A stranger finding Places 1 and 2 now spends too; Places 1 and 3
    // already did: the margin is 1 either way.
    assert_eq!(stranger_margin(&s, &plain), 1);
    assert_eq!(stranger_margin(&s, &written), 1);
    // The written password reads back with the plan.
    let back = Answers::from_text(&s, &written.to_text()).expect("the plan reads back");
    assert!(route_met(&s, &back, &paper));
}

/// A plan saved before the layers reads as one layer, "Access layer 1",
/// able to spend with every place, every vault's password and every
/// passphrase: met.
#[test]
fn a_plan_saved_before_layers_is_one_layer_that_spends_with_everything() {
    let mut s = shape(2, 3);
    s.seeds[1].passphrase = true;
    let saved = Answers::preset(&s, Preset::PaperVault).to_text();
    let a = Answers::from_text(&s, &saved).expect("the old plan reads");
    let layers = a.access_layers(&s);
    assert_eq!(layers.len(), 1);
    let r = &layers[0].routes;
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].places, (0..a.places).collect::<Vec<_>>());
    assert_eq!(r[0].any, a.places);
    assert_eq!(r[0].passwords, vec![0, 1, 2]);
    assert_eq!(r[0].passphrases, vec![1]);
    assert!(layer_met(&s, &a, &layers[0]));
    assert_eq!(layer_name(&[], 0), "Access layer 1");
}

/// A 2-of-3 with Seed 1's words at Place 1, Seed 2's words and the sheet
/// at Place 2, Seed 3's words at Place 3. Access layer 1's paper route is
/// Places 1 and 2, Access layer 2's Places 2 and 3: the one copy of Seed
/// 2 and the one sheet at Place 2 serve both, and both can spend from
/// paper. The plan keeps four things whatever the layers.
#[test]
fn two_layers_whose_paper_routes_share_place_2_are_both_met_by_its_one_copy() {
    let s = shape(2, 3);
    let mut a = drawn(
        &s,
        &[
            &[What::Words(0)],
            &[What::Words(1), What::Sheet],
            &[What::Words(2)],
        ],
        &[],
    );
    let before = kept(&s, &a);
    a.layers = vec![
        layer(vec![paper(&[0, 1], 2)]),
        layer(vec![paper(&[1, 2], 2)]),
    ];
    let reach = Reach::of(&s, &a);
    assert!(reach.paper_met(&a.layers[0]));
    assert!(reach.paper_met(&a.layers[1]));
    assert!(every_layer_on_paper(&s, &a));
    assert_eq!(kept(&s, &a), before);
    assert_eq!(before.copies, 4);
    // Places 1 and 3 alone have no description: a layer whose paper
    // route is those two cannot spend from paper.
    a.layers.push(layer(vec![paper(&[0, 2], 2)]));
    assert!(!every_layer_on_paper(&s, &a));
}

/// A paper route counts no stick: on the 2-of-3 Paper and vault preset
/// with Vault 2's stick at Place 1 too, Place 1 knowing Vault 2's
/// password spends, but from Place 1's paper alone it does not. A layer
/// with no paper route cannot spend from paper, met or not; so does a
/// plan saved before the layers, whose one layer has none.
#[test]
fn a_paper_route_spends_from_paper_alone() {
    let s = shape(2, 3);
    let mut a = Answers::preset(&s, Preset::PaperVault);
    a.sticks[1][0] = true;
    let reach = Reach::of(&s, &a);
    assert!(reach.route_met(&route(&[0], 1, &[1])));
    assert!(!reach.route_met(&Route {
        paper: true,
        ..route(&[0], 1, &[1])
    }));
    assert!(reach.route_met(&paper(&[0, 1], 2)));
    let no_paper = layer(vec![route(&[0], 1, &[1])]);
    assert!(reach.layer_met(&no_paper));
    assert!(!reach.paper_met(&no_paper));
    assert!(!every_layer_on_paper(&s, &a));
}

const LAYER_NAME: &str = "Grandchildren";

fn two_layers() -> Vec<Layer> {
    vec![
        layer(vec![route(&[0], 1, &[0]), paper(&[0, 1, 2], 2)]),
        Layer {
            routes: vec![Route {
                places: vec![1, 2],
                any: 2,
                passwords: Vec::new(),
                passphrases: vec![1],
                paper: true,
                written: Vec::new(),
            }],
        },
    ]
}

/// Two layers, one with two routes, kept in the plan's text and read back
/// as they were, with no layer's name in the text; the checks read the
/// same from the plan read back.
#[test]
fn the_layers_read_back_from_the_plans_text_without_their_names() {
    let mut s = shape(2, 3);
    s.seeds[1].passphrase = true;
    let mut a = Answers::preset(&s, Preset::PaperVault);
    a.layers = two_layers();
    let text = a.to_text();
    assert!(!text.contains(LAYER_NAME));
    let back = Answers::from_text(&s, &text).expect("the plan reads back");
    assert_eq!(back.layers, a.layers);
    assert_eq!(back.access_layers(&s), two_layers());
    let (before, after) = (Reach::of(&s, &a), Reach::of(&s, &back));
    for l in &a.layers {
        assert_eq!(before.layer_met(l), after.layer_met(l));
    }
    assert_eq!(before.stranger_margin(), after.stranger_margin());
    // A route naming a place the plan does not have is not a plan.
    let bad = text.replace("route 1 2 1,2", "route 1 2 1,7");
    assert!(Answers::from_text(&s, &bad).is_none());
}

/// The vault's plan record keeps each layer's name beside the places'
/// names; written into a slot and read back, the names and the layers
/// are as they were, and the answers' field holds no name.
#[test]
fn the_vaults_plan_record_keeps_the_layers_names() {
    let mut s = shape(2, 3);
    s.seeds[1].passphrase = true;
    let mut a = Answers::preset(&s, Preset::PaperVault);
    a.layers = two_layers();
    let names = vec![String::new(), LAYER_NAME.to_string()];
    let mut record = Record::new(kind::PLAN)
        .with(field::PLAN_WALLET, b"wallet")
        .with(field::PLAN_ANSWERS, a.to_text().as_bytes());
    record.push(field::PLAN_PLACE, b"Home");
    push_layer_names(&mut record, &names, a.layers.len());
    let slot = Contents {
        records: vec![record],
    }
    .encode(4096)
    .expect("the record fits a slot");
    let read = Contents::decode(&slot).expect("the slot reads back");
    let r = read.of(kind::PLAN).next().expect("a plan record").1;
    assert_eq!(layer_names(r), names);
    assert_eq!(layer_name(&layer_names(r), 0), "Access layer 1");
    assert_eq!(layer_name(&layer_names(r), 1), LAYER_NAME);
    let text = r.text(field::PLAN_ANSWERS).unwrap();
    assert!(!text.contains(LAYER_NAME));
    let back = Answers::from_text(&s, text).expect("the plan reads back");
    assert_eq!(back.layers, two_layers());
}

// ----- The plan saved again with a vault open keeps the layers' names -----

fn device() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
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

fn made_and_unlocked(app: &mut Faraday) {
    const PHRASE: &str = "test phrase";
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

fn press_offered(app: &mut Faraday, action: Action) {
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        if app.offers(action) {
            app.press(action);
            return;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    panic!("{action:?} is not offered");
}

fn holds(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|w| w == needle)
}

fn plan_record(app: &Faraday) -> Record {
    app.vaults.open[0]
        .contents
        .of(kind::PLAN)
        .next()
        .expect("a plan record")
        .1
        .clone()
}

/// A layer named in the vault's plan record keeps its name when the plan
/// is saved again, and nothing kept across a lock holds it.
#[test]
fn saving_the_plan_again_keeps_a_layers_name_in_the_vault_alone() {
    let mut app = device();
    made_and_unlocked(&mut app);
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().fingerprint();
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    press_offered(&mut app, Action::BPreset(1));
    app.press(Action::BQ(qstep::PLAN));
    press_offered(&mut app, Action::BChecklist);

    // The layer named while the vault was open.
    let mut record = plan_record(&app);
    record.push(field::PLAN_LAYER, LAYER_NAME.as_bytes());
    let r = app.vaults.open[0]
        .contents
        .records
        .iter_mut()
        .find(|r| r.kind == kind::PLAN)
        .unwrap();
    *r = record;

    // The key checked and saved into the vault: the plan is saved again.
    if app.backup_items().contains(&faraday_core::bstep::BLANK) {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    app.press(Action::BReveal);
    app.storage(StorageEvent::Cameras(Vec::new()));
    press_offered(&mut app, Action::BCheck);
    let words = testkit::test_words(testkit::TEST_SEEDS[1].0);
    let digits = osk_codec::seedqr::to_digits(
        &osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, &words).unwrap(),
    );
    type_text(
        &mut app,
        std::str::from_utf8(digits.expose().as_bytes()).unwrap(),
    );
    press_offered(&mut app, Action::BStep(faraday_core::bstep::VAULT));
    press_offered(&mut app, Action::BVaultSave(0));
    let record = plan_record(&app);
    assert_eq!(layer_names(&record), vec![LAYER_NAME.to_string()]);
    assert!(!holds(
        record.field(field::PLAN_ANSWERS).unwrap(),
        LAYER_NAME.as_bytes()
    ));

    app.press(Action::Lock);
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k;
        }
    }
    for (name, bytes) in &kept {
        assert!(
            !holds(bytes, LAYER_NAME.as_bytes()),
            "{name}, kept on locking, holds the layer's name"
        );
    }
}

// ----- Work it out (§20.3, §20.8) -----

use faraday_core::access::kept_in;
use faraday_core::layers::{LayersPress, conflicts};
use faraday_core::work_out::{BUDGET, Outcome, work_out};

/// Work it out on plan `a` for `layers`, the result put on the plan.
fn worked(s: &Shape, a: &Answers, layers: &[Layer]) -> (Outcome, Answers) {
    let out = work_out(s, a, layers, BUDGET).expect("a route can be met");
    let mut b = a.clone();
    b.layers = layers.to_vec();
    b.set_map(out.map.clone());
    (out, b)
}

/// Every layer of `b` met by every route, each with its paper route.
fn all_met(s: &Shape, b: &Answers) -> bool {
    let reach = Reach::of(s, b);
    b.layers
        .iter()
        .all(|l| reach.layer_met(l) && reach.paper_met(l))
}

/// What plan `b` keeps: copies on paper, sticks, vaults.
fn keeps(s: &Shape, b: &Answers) -> (usize, usize, usize) {
    let k = kept_in(&map(s, b));
    (k.copies, k.sticks, k.vaults)
}

/// Each preset with the layer a fresh plan starts with (one paper route,
/// any m of the preset's places), worked out. By hand:
///
/// - One key, two places, any 1 of them: each place alone must spend, so
///   the seed's words are at both and a stranger at either spends (margin
///   0, §20.12 item 5: at most k − 1), and sees the balance (0). Two
///   copies.
/// - 2 of 3, any 2 of 3 places: a stranger finding a route's 2 places
///   spends, so the margin is at most 1; one seed and one share at each
///   place reach it (one place: one seed; any two: two seeds and two
///   shares, each share two of the three keys, which cover all three).
///   One place sees no balance: a share holds two keys (balance 1, no
///   more than the margin). Fewer copies fail: a place without a seed
///   leaves its pairs needing two seeds at the other place, four seeds in
///   all; a place without a share leaves its pairs needing the
///   description whole at the other, which then sees the balance alone.
///   Six copies.
/// - 3 of 5, any 3 of 5: margin at most 2; a seed and a share at each
///   place reach it (two places: two seeds). A share holds three keys,
///   shares i and i + 2 together all five, so some two places see the
///   balance: 1, and 2 cannot be had (a place's missing keys are two
///   neighbours, and five places cannot each meet every other's and no
///   three share one). Seeds: with one at each place, five; a place
///   with none makes every two of the others hold three, seven at least.
///   Shares: a place with none leaves every two of the other four
///   to cover the keys, which one share each cannot (shares cover in
///   pairs only two apart, and no three shares are pairwise so); five at
///   least. Ten copies.
///
/// The vaults a preset made are not needed by a paper route and go.
#[test]
fn every_preset_worked_out_meets_its_layer_with_the_margin_worked_by_hand() {
    // (m, n): margin, balance, copies.
    let hand = [
        ((1, 1), (0, 0, 2)),
        ((2, 3), (1, 1, 6)),
        ((3, 5), (2, 1, 10)),
    ];
    for ((m, n), (margin, balance, copies)) in hand {
        let s = shape(m, n);
        for preset in Preset::ALL {
            let a = Answers::preset(&s, preset);
            let layers = a.layers_shown(&s);
            assert_eq!(layers.len(), 1);
            assert!(layers[0].routes[0].paper);
            let (out, b) = worked(&s, &a, &layers);
            let what = format!("{m} of {n}, {preset:?}");
            assert!(all_met(&s, &b), "{what}: met");
            assert_eq!(stranger_margin(&s, &b), margin, "{what}: margin");
            assert_eq!(balance_margin(&s, &b), balance, "{what}: balance");
            assert_eq!(keeps(&s, &b), (copies, 0, 0), "{what}: kept");
            assert_eq!(out.margin, margin);
            assert!(!out.stopped, "{what}: every placement tried");
        }
    }
}

/// The worst preset case (§20.8): a 3 of 5, Access layer 1 through Vault
/// 1 at Place 1 or any 3 of the five places on paper, Access layer 2
/// through Vault 2 at Place 2 or Place 3, or any 3 of Places 2 to 5 on
/// paper, Access layer 3 any 3 of the five on paper.
fn worst() -> Vec<Layer> {
    vec![
        layer(vec![route(&[0], 1, &[0]), paper(&[0, 1, 2, 3, 4], 3)]),
        layer(vec![route(&[1, 2], 1, &[1]), paper(&[1, 2, 3, 4], 3)]),
        layer(vec![paper(&[0, 1, 2, 3, 4], 3)]),
    ]
}

/// The worst case worked out, by hand: the paper routes are any 3, so
/// the margin is at most 2, and a seed and a share at each place give it
/// (as the 3 of 5 above, Places 2 to 5 covering for Access layer 2's
/// paper route too, since any three shares cover the keys). Places 1, 2
/// and 3 alone must spend for the routes through the vaults, from what a
/// stranger there cannot read: Vault 1's stick at Place 1, Vault 2's at
/// Places 2 and 3, each vault holding the seeds and the description.
/// Balance 1 as before. Ten copies, three sticks, two vaults; and the
/// search ends within its budget, every placement tried.
#[test]
fn the_worst_preset_case_is_worked_out_within_the_budget() {
    let s = shape(3, 5);
    let a = Answers::preset(&s, Preset::PaperVault);
    let (out, b) = worked(&s, &a, &worst());
    assert!(all_met(&s, &b));
    assert_eq!(stranger_margin(&s, &b), 2);
    assert_eq!(balance_margin(&s, &b), 1);
    assert_eq!(keeps(&s, &b), (10, 3, 2));
    assert!(out.steps <= BUDGET, "{} steps", out.steps);
    assert!(!out.stopped);
}

/// The same plan worked out twice gives the same map, steps and all.
#[test]
fn work_it_out_gives_the_same_result_on_every_run() {
    let s = shape(3, 5);
    let a = Answers::preset(&s, Preset::PaperVault);
    let first = work_out(&s, &a, &worst(), BUDGET);
    for _ in 0..3 {
        assert_eq!(work_out(&s, &a, &worst(), BUDGET), first);
    }
    // Stopped short by a smaller budget, the best found is the same too.
    let short = work_out(&s, &a, &worst(), 1000).unwrap();
    assert!(short.stopped);
    assert_eq!(work_out(&s, &a, &worst(), 1000).unwrap(), short);
}

/// Two layers of a 2 of 3, Access layer 1 any 2 of Places 1 and 2 on
/// paper, Access layer 2 any 2 of Places 2 and 3: both need Place 2. By
/// hand: margin 1 (a route's two places spend). Each place keeps a share
/// (one alone would leave a pair needing the description whole at its
/// other place, which then sees the balance alone): three shares, any two
/// covering the keys, balance 1. The seeds: Place 2 keeps two of them,
/// which with one share cannot spend alone, and serve both layers; Places
/// 1 and 3 need none. Five copies, Place 2's three serving both layers
/// once each (a seed at each place would be six).
#[test]
fn two_layers_sharing_a_places_paper_are_met_by_one_copy() {
    let s = shape(2, 3);
    let a = Answers::preset(&s, Preset::Paper);
    let layers = vec![
        layer(vec![paper(&[0, 1], 2)]),
        layer(vec![paper(&[1, 2], 2)]),
    ];
    let (_, b) = worked(&s, &a, &layers);
    assert!(all_met(&s, &b));
    assert_eq!(stranger_margin(&s, &b), 1);
    assert_eq!(balance_margin(&s, &b), 1);
    assert_eq!(keeps(&s, &b), (5, 0, 0));
    let at = |p: usize| -> Vec<What> {
        map(&s, &b)
            .iter()
            .filter(|x| x.at == At::Place(p))
            .flat_map(|x| x.holds.iter().map(|(w, _)| *w))
            .collect()
    };
    let words = |p: usize| at(p).iter().filter(|w| matches!(w, What::Words(_))).count();
    assert_eq!((words(0), words(1), words(2)), (0, 2, 0), "{:?}", at(1));
}

/// A layer with no paper route is a conflict, and its first resolution
/// gives it one.
#[test]
fn a_layer_without_a_paper_route_is_a_conflict() {
    let s = shape(2, 3);
    let a = Answers::preset(&s, Preset::PaperVault);
    let layers = vec![
        layer(vec![paper(&[0, 1, 2], 2)]),
        layer(vec![route(&[0, 1, 2], 2, &[0])]),
    ];
    let name = |p: usize| format!("Place {}", p + 1);
    let found = conflicts(&s, &a, &layers, &name);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, "Access layer 2 has no paper route");
    assert_eq!(found[0].fixes[0].1, LayersPress::AddPaperRoute(1));
}

/// A route's vault passwords written down, paper route or not, read back
/// with the plan.
#[test]
fn a_routes_written_passwords_read_back_with_the_plan() {
    let s = shape(2, 3);
    let mut a = Answers::preset(&s, Preset::PaperVault);
    a.layers = vec![layer(vec![
        Route {
            written: vec![0, 2],
            ..route(&[0, 1], 1, &[1])
        },
        Route {
            written: vec![1],
            ..paper(&[0, 1, 2], 2)
        },
    ])];
    let back = Answers::from_text(&s, &a.to_text()).expect("the plan reads back");
    assert_eq!(back.layers, a.layers);
}
