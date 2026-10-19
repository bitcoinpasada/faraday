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
        kinds: None,
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
                kinds: None,
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

// ----- What each place of a route holds (§20.13, L2b) -----

use faraday_core::access::{Described, KindVault, Kinds, SeedForm};
use faraday_core::work_out::{Fit, Why, fit, problems};

/// Kinds: `seeds` per place in `form`, the description `d`, vault `v`.
fn kinds(seeds: usize, form: SeedForm, d: Described, v: KindVault) -> Option<Kinds> {
    Some(Kinds {
        seeds,
        form,
        description: d,
        vault: v,
    })
}

/// What place `p` of plan `b` keeps, on paper and on its sticks.
fn at_place(s: &Shape, b: &Answers, p: usize) -> Vec<What> {
    map(s, b)
        .iter()
        .filter(|x| matches!(x.at, At::Place(q) | At::Stick(q, _) if q == p))
        .flat_map(|x| x.holds.iter().map(|(w, _)| *w))
        .collect()
}

/// L2's results, recorded from L2's search before the kinds came: for
/// each wallet, preset and set of layers (the one a fresh plan starts
/// with; the worst preset case; two layers sharing Place 2), the steps
/// taken and the score found. Every route's kinds unset, the search
/// walks the same placements in the same order and finds the same.
#[test]
fn unset_kinds_give_l2s_results_unchanged() {
    let share = vec![
        layer(vec![paper(&[0, 1], 2)]),
        layer(vec![paper(&[1, 2], 2)]),
    ];
    // (m, n), layers: steps, margin, balance, kept, unsealed.
    let l2 = [
        ((1, 1), "fresh", (3, 0, 0, 2, 2)),
        ((2, 3), "fresh", (114, 1, 1, 6, 3)),
        ((2, 3), "share", (413, 1, 1, 5, 2)),
        ((3, 5), "fresh", (15_070, 2, 1, 10, 5)),
        ((3, 5), "worst", (320_119, 2, 1, 15, 5)),
        ((3, 5), "share", (1_783, 1, 1, 6, 3)),
    ];
    for ((m, n), which, want) in l2 {
        let s = shape(m, n);
        for preset in Preset::ALL {
            let a = Answers::preset(&s, preset);
            let layers = match which {
                "fresh" => a.layers_shown(&s),
                "worst" => worst(),
                _ => share.clone(),
            };
            assert!(
                layers
                    .iter()
                    .flat_map(|l| &l.routes)
                    .all(|r| r.kinds.is_none())
            );
            let o = work_out(&s, &a, &layers, BUDGET).unwrap();
            assert_eq!(
                (o.steps, o.margin, o.balance, o.kept, o.unsealed, o.stopped),
                (want.0, want.1, want.2, want.3, want.4, false),
                "{m} of {n}, {preset:?}, {which}"
            );
        }
    }
}

/// §20.13 item 4's example in Back up, its 2 of 3 made: Access layer 1
/// at Place 1 with a vault there whose password it remembers, nothing
/// else said for that place; Access layer 2 on paper, any 2 of Places 1
/// to 3, a seed's words and a part of the description at each. By hand:
/// each place keeps at least one seed's words and one share. Any two
/// places must give two seeds and the whole description: the three
/// seeds at three places, one each, give any pair two different seeds,
/// and three different shares (each share two of the three keys) give
/// any pair all three keys; so one of each per place does it, six
/// copies, and the kinds allow no fewer. Place 1 keeps Vault 1's stick;
/// the vault, its password written nowhere, holds the seeds and the
/// description, so Access layer 1 spends from Place 1 alone. A stranger
/// at one place has one seed and a part (and at Place 1 a vault it cannot
/// open): no spend, no balance; at two places of the paper route a
/// stranger spends, so both margins are 1.
#[test]
fn the_vault_and_paper_example_places_a_seed_and_a_part_at_each_place() {
    let s = shape(2, 3);
    let a = Answers::preset(&s, Preset::Paper);
    let layers = vec![
        layer(vec![Route {
            kinds: kinds(0, SeedForm::Paper, Described::None, KindVault::Remembered),
            ..route(&[0], 1, &[0])
        }]),
        layer(vec![Route {
            kinds: kinds(1, SeedForm::Paper, Described::Part, KindVault::None),
            ..paper(&[0, 1, 2], 2)
        }]),
    ];
    assert!(problems(&s, &layers).is_empty());
    let Fit::Placed(out) = fit(&s, &a, &layers, BUDGET) else {
        panic!("the example is placed");
    };
    let mut b = a.clone();
    b.layers = layers.clone();
    b.set_map(out.map.clone());
    let reach = Reach::of(&s, &b);
    assert!(b.layers.iter().all(|l| reach.layer_met(l)));
    assert_eq!(stranger_margin(&s, &b), 1);
    assert_eq!(balance_margin(&s, &b), 1);
    assert_eq!(keeps(&s, &b), (6, 1, 1));
    let mut seeds = Vec::new();
    for p in 0..3 {
        let here = at_place(&s, &b, p);
        let words: Vec<usize> = here
            .iter()
            .filter_map(|w| match w {
                What::Words(i) => Some(*i),
                _ => None,
            })
            .collect();
        let shares = here.iter().filter(|w| matches!(w, What::Share(_))).count();
        assert_eq!((words.len(), shares), (1, 1), "Place {}: {here:?}", p + 1);
        seeds.extend(words);
        assert_eq!(here.contains(&What::VaultStick(0)), p == 0);
    }
    seeds.sort_unstable();
    assert_eq!(seeds, vec![0, 1, 2], "a different seed at each place");
    let vault: Vec<What> = map(&s, &b)
        .iter()
        .filter(|x| x.at == At::Vault(0))
        .flat_map(|x| x.holds.iter().map(|(w, _)| *w))
        .collect();
    assert!(vault.contains(&What::Wallet));
    assert!(vault.iter().filter(|w| matches!(w, What::Seed(_))).count() >= 2);
}

/// A route of one place, a seed's words and the whole description there,
/// remembering the seed's passphrase: in a single key with a passphrase,
/// one copy of the words at that place, and a stranger who finds it lacks
/// the passphrase (margin 1, all the places there are). In a single key
/// with no passphrase the same words alone spend there: margin 0. So of
/// the two shapes, the one with a passphrase keeps the stranger out.
#[test]
fn one_place_with_a_remembered_passphrase_is_placed_out_of_a_strangers_reach() {
    let mut with = shape(1, 1);
    with.seeds[0].passphrase = true;
    let layers = vec![layer(vec![Route {
        passphrases: vec![0],
        kinds: kinds(1, SeedForm::Paper, Described::Whole, KindVault::None),
        ..paper(&[0], 1)
    }])];
    let mut a = Answers::preset(&with, Preset::Paper);
    a.set_map(vec![Spot {
        at: At::Place(0),
        holds: Vec::new(),
    }]);
    let Fit::Placed(out) = fit(&with, &a, &layers, BUDGET) else {
        panic!("placed with a passphrase");
    };
    assert_eq!((out.margin, out.kept), (1, 1));
    let mut b = a.clone();
    b.set_map(out.map.clone());
    assert_eq!(at_place(&with, &b, 0), vec![What::Words(0)]);

    let plain = shape(1, 1);
    let a = Answers::preset(&plain, Preset::Paper);
    let Fit::Placed(out) = fit(&plain, &a, &layers, BUDGET) else {
        panic!("placed without a passphrase");
    };
    assert_eq!(out.margin, 0);
}

/// Two places of one seed each against a 3 of 5 in Back up: any two give
/// two seeds, and the wallet needs three. The conflict names the route
/// and why; its resolutions are two seeds at each place (2 × 2 ≥ 3) and
/// any three places (3 × 1 = 3, a place added). Work it out leaves the
/// route out: nothing is placed for it. In a 2 of 3 the same route is no
/// conflict (2 × 1 = 2, any two of two shares cover the three keys).
#[test]
fn two_places_of_one_seed_each_against_a_three_of_five_is_a_conflict_naming_the_route() {
    let s = shape(3, 5);
    let a = Answers::preset(&s, Preset::Paper);
    let layers = vec![layer(vec![Route {
        kinds: kinds(1, SeedForm::Paper, Described::Part, KindVault::None),
        ..paper(&[0, 1], 2)
    }])];
    let found = problems(&s, &layers);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].at, Some((0, 0)));
    assert_eq!(
        found[0].why,
        Why::Short {
            any: 2,
            per: 1,
            needs: 3
        }
    );
    let name = |p: usize| format!("Place {}", p + 1);
    let lines = conflicts(&s, &a, &layers, &name);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(
        lines[0].line,
        "Access layer 1, route 1: any 2 places with 1 each give 2 seeds; the wallet needs 3"
    );
    assert_eq!(lines[0].fixes[0].1, LayersPress::KindsSeeds(0, 0, 2));
    assert_eq!(lines[0].fixes[1].1, LayersPress::KindsAny(0, 0, 3));
    assert!(matches!(fit(&s, &a, &layers, BUDGET), Fit::Conflicts(_)));
    assert!(work_out(&s, &a, &layers, BUDGET).is_none());
    // The shape these kinds need, and in that shape no conflict.
    assert_eq!(
        faraday_core::layers::shape_needed(&s, &a, &layers, true).as_deref(),
        Some("These routes need a 2 of 3; this wallet is a 3 of 5")
    );
    let two = shape(2, 3);
    let b = Answers::preset(&two, Preset::Paper);
    assert!(problems(&two, &layers).is_empty());
    assert!(matches!(fit(&two, &b, &layers, BUDGET), Fit::Placed(o) if o.margin == 1));
}

/// Two seeds and the whole description at each place of a 2 of 3's
/// paper route, any 2 of 3: one place alone holds a quorum and the
/// description, so a stranger who finds it spends while the route asks
/// for two. A conflict naming the route, resolved by one seed at each
/// place or a part of the description there; with one seed each, none.
#[test]
fn a_place_whose_kinds_let_a_stranger_spend_alone_is_a_conflict() {
    let s = shape(2, 3);
    let a = Answers::preset(&s, Preset::Paper);
    let mut layers = vec![layer(vec![Route {
        kinds: kinds(2, SeedForm::Paper, Described::Whole, KindVault::None),
        ..paper(&[0, 1, 2], 2)
    }])];
    let name = |p: usize| format!("Place {}", p + 1);
    let lines = conflicts(&s, &a, &layers, &name);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(
        lines[0].line,
        "Access layer 1, route 1: a stranger who finds one of its places can spend"
    );
    let fixes: Vec<LayersPress> = lines[0].fixes.iter().map(|f| f.1).collect();
    assert_eq!(
        fixes,
        vec![
            LayersPress::KindsSeeds(0, 0, 1),
            LayersPress::KindsDescription(0, 0, Described::Part)
        ]
    );
    layers[0].routes[0].kinds = kinds(1, SeedForm::Paper, Described::Whole, KindVault::None);
    assert!(conflicts(&s, &a, &layers, &name).is_empty());
}

/// A 2 of 3, Access layer 1 off paper, any 2 of Places 1 to 3, a seed as
/// a file on a stick and a part at each place; and its paper route, any
/// 2 of the three, its kinds unset. By hand: the kinds force a seed's
/// file and a share at each place (three files on three sticks, three
/// shares, all different shares so any two cover the keys). The paper
/// route counts no file: any two places need two seeds' words, so three
/// words at least (two at one place leave the other two places with at
/// most one), one per place. A stranger at one place has its words and
/// its file, at most two seeds, and one share: not the description, so
/// no spend and no balance; two places spend (the paper route is any
/// 2). Margins 1 and 1; nine copies, three sticks.
#[test]
fn seeds_as_files_go_on_a_stick_at_each_place() {
    let s = shape(2, 3);
    let a = Answers::preset(&s, Preset::Paper);
    let layers = vec![layer(vec![
        Route {
            kinds: kinds(1, SeedForm::Stick, Described::Part, KindVault::None),
            ..route(&[0, 1, 2], 2, &[])
        },
        paper(&[0, 1, 2], 2),
    ])];
    let (out, b) = worked(&s, &a, &layers);
    assert!(all_met(&s, &b));
    assert_eq!((out.margin, out.balance), (1, 1));
    assert_eq!(keeps(&s, &b), (9, 3, 0));
    for p in 0..3 {
        let stick: Vec<What> = map(&s, &b)
            .iter()
            .filter(|x| matches!(x.at, At::Stick(q, _) if q == p))
            .flat_map(|x| x.holds.iter().map(|(w, _)| *w))
            .collect();
        assert_eq!(stick.len(), 1, "Place {}: {stick:?}", p + 1);
        assert!(matches!(stick[0], What::SeedFile(_)));
    }
}

/// The worst preset case with kinds (§20.8): its three paper routes say
/// a seed's words and a part of the description at each place. By hand
/// this is what L2 found for it unset (a seed and a share at each place,
/// the vaults' sticks at Places 1, 2 and 3): the same margins and what is
/// kept, in no more steps, every placement tried.
#[test]
fn the_worst_preset_case_with_kinds_is_worked_out_within_the_budget() {
    let s = shape(3, 5);
    let a = Answers::preset(&s, Preset::PaperVault);
    let mut layers = worst();
    for r in layers.iter_mut().flat_map(|l| l.routes.iter_mut()) {
        if r.paper {
            r.kinds = kinds(1, SeedForm::Paper, Described::Part, KindVault::None);
        }
    }
    let (out, b) = worked(&s, &a, &layers);
    assert!(all_met(&s, &b));
    assert_eq!(stranger_margin(&s, &b), 2);
    assert_eq!(balance_margin(&s, &b), 1);
    assert_eq!(keeps(&s, &b), (10, 3, 2));
    assert!(!out.stopped);
    assert!(out.steps <= 320_119, "{} steps", out.steps);
}

/// A route's kinds read back with the plan's text, on and off paper; a
/// route written before the kinds reads as unset; a paper route whose
/// kinds say a stick or a vault is not a plan.
#[test]
fn a_routes_kinds_read_back_with_the_plan() {
    let s = shape(2, 3);
    let mut a = Answers::preset(&s, Preset::PaperVault);
    a.layers = vec![layer(vec![
        Route {
            kinds: kinds(0, SeedForm::Stick, Described::Whole, KindVault::Written),
            written: vec![1],
            ..route(&[0], 1, &[])
        },
        Route {
            kinds: kinds(1, SeedForm::Paper, Described::Part, KindVault::None),
            ..paper(&[0, 1, 2], 2)
        },
        paper(&[1, 2], 2),
    ])];
    let text = a.to_text();
    let back = Answers::from_text(&s, &text).expect("the plan reads back");
    assert_eq!(back.layers, a.layers);
    assert_eq!(back.layers[0].routes[2].kinds, None);
    let bad = text.replace(
        "paper kinds 1 words part none",
        "paper kinds 1 file part none",
    );
    assert_ne!(bad, text);
    assert!(Answers::from_text(&s, &bad).is_none());
}

// ---- Create's Suggest a wallet (§20.12 item 2, §20.14 item 3) ----

/// The worst preset case said on Create's Access layers card, before any
/// wallet: five places, Access layer 1 through Vault 1 at Place 1 or any 3
/// of the five on paper, Access layer 2 through Vault 2 at Place 2 or 3 or
/// any 3 of Places 2 to 5 on paper, Access layer 3 any 3 of the five.
fn worst_in_create() -> faraday_core::create_plan::CreatePlan {
    let mut p = faraday_core::create_plan::CreatePlan::fresh();
    let s = faraday_core::create_plan::planning();
    p.answers.add_place(&s);
    p.answers.add_place(&s);
    p.answers.layers = worst();
    p
}

/// Every shape tried shares one budget (§20.8's one second): the four
/// together take no more than [`BUDGET`] steps. Its paper routes are any 3
/// of their places, so no shape's stranger margin passes 2 (§20.12 item
/// 5); a 3 of 5 with a seed and a share at each place reaches it, as Back
/// up's worst case found, and comes first. A single key's seed at any
/// one place spends: margin 0, last of those that fit. Nothing remembers
/// a passphrase, so a single key with one cannot sign. The same on every
/// run.
#[test]
fn suggest_a_wallet_shares_one_budget_among_the_shapes_and_ranks_by_margin() {
    use faraday_core::create_plan::{WalletShape, suggest};
    let p = worst_in_create();
    let (fits, steps) = suggest(&p.answers);
    assert!(steps <= BUDGET, "{steps} steps");
    assert_eq!(fits[0].shape, WalletShape::ThreeOfFive);
    assert!(fits[0].fits);
    assert_eq!(fits[0].margin, 2);
    let single = fits
        .iter()
        .find(|f| f.shape == WalletShape::Single)
        .unwrap();
    assert!(single.fits && single.margin == 0, "{single:?}");
    assert!(fits.iter().all(|f| f.margin <= 2));
    let pass = fits
        .iter()
        .find(|f| f.shape == WalletShape::SinglePassphrase)
        .unwrap();
    assert!(!pass.fits);
    assert_eq!(suggest(&p.answers), (fits, steps), "the same on every run");
}

// ----- The layer view (§20.7, L3) -----

use faraday_core::board::{BoardPress as BP, View};
use faraday_core::cstep;

/// Back up of a 2 of 3 made in Create over three keys typed in here, on
/// the Paper and vault preset, with two layers said on the Access layers
/// card and Work it out run: Access layer 1 by a paper route, any 2 of
/// Places 1, 2 and 3, and by Place 1 alone knowing Vault 1's password;
/// Access layer 2 by a paper route, any 2 of Places 2 and 3.
fn two_layers_worked() -> Faraday {
    let mut app = device();
    for n in 0..3 {
        app.press(Action::Entry(None));
        type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        app.press(Action::EntryAdd);
    }
    testkit::create_knowing(&mut app);
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .unwrap() as u8;
    app.press(Action::CKind(multi));
    for slot in 0..3u8 {
        let fp = app.session.keys[usize::from(slot)].fingerprint().0;
        app.press(Action::CSlotHere(slot, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(0));
    app.press(Action::BPreset(1));
    app.press(Action::BQ(qstep::LAYERS));
    let lp = |app: &mut Faraday, p: LayersPress| app.press(Action::Layers(p));
    lp(&mut app, LayersPress::AddRoute(0));
    lp(&mut app, LayersPress::Place(0, 1, 1));
    lp(&mut app, LayersPress::Place(0, 1, 2));
    lp(&mut app, LayersPress::Password(0, 1, 0));
    lp(&mut app, LayersPress::AddLayer);
    lp(&mut app, LayersPress::Place(1, 0, 0));
    lp(&mut app, LayersPress::WorkItOut);
    assert_eq!(
        app.backup.as_ref().unwrap().q,
        Some(qstep::BOARD),
        "the board opens on the result"
    );
    app
}

fn view(app: &mut Faraday, v: View) {
    app.press(Action::Board(BP::View(v)));
}

/// Every text drawn down the page, a frame at a time from its top.
fn frames_down(app: &mut Faraday) -> Vec<Vec<String>> {
    app.event(Event::Scroll {
        x: 600,
        y: 400,
        dy: -20_000,
    });
    let mut out = Vec::new();
    for _ in 0..30 {
        app.settle();
        out.push(app.drawn_texts());
        app.event(Event::Scroll {
            x: 600,
            y: 400,
            dy: 150,
        });
    }
    out
}

/// The value drawn after the check `label` in the group headed `head`
/// (the last text drawn that is a layer's or a route's head before it),
/// anywhere down the page.
fn check(app: &mut Faraday, head: &str, label: &str) -> String {
    for texts in frames_down(app) {
        let mut under = String::new();
        for (k, t) in texts.iter().enumerate() {
            if t.starts_with("Access layer") {
                under = t.clone();
            }
            if t == label && (head.is_empty() || under == head) {
                return texts.get(k + 1).cloned().unwrap_or_default();
            }
        }
    }
    panic!("no {label:?} under {head:?}");
}

/// How many times `want` is drawn in the first frame down the page that
/// draws `band`.
fn drawn_with(app: &mut Faraday, band: &str, want: &str) -> Option<usize> {
    frames_down(app)
        .into_iter()
        .find(|t| t.iter().any(|x| x == band))
        .map(|t| t.iter().filter(|x| *x == want).count())
}

/// Each layer's view of the two layers above, its checks worked by hand.
///
/// Work it out's placement, by hand: Access layer 1's paper route needs
/// any two of the three places to spend from paper, so a stranger finding
/// two places spends (the margin is at most 1); one seed and one share at
/// each place, the seeds and the shares all different, give that with
/// the fewest copies (as the 2 of 3 preset worked out above). Route 2,
/// Place 1 alone with Vault 1's password, must spend from what a stranger
/// at Place 1 cannot read: Vault 1's stick at Place 1 alone, the vault
/// holding the seeds and the description. Access layer 2's two places
/// are two of those three.
///
/// Access layer 1, route 1 · paper: any two places give two seeds and two
/// shares, which cover all three keys: can spend; one place gives one
/// seed, so it needs 2 of its 3; any two see the balance. Route 2: Place 1
/// opens Vault 1, which holds every seed and the description: can spend,
/// 1 of its 1, sees the balance. Places it can lose, by some route: any
/// one lost leaves either the other two (route 1) or Place 1 (route 2);
/// Places 1 and 2 lost leave Place 3 alone, one seed, and no Place 1: 1.
///
/// Access layer 2: Places 2 and 3 give two seeds and two shares: can
/// spend, 2 of its 2, sees the balance; either lost leaves one seed: 0.
/// It knows no password and none is written at its places, so Vault 1 is
/// shut to it; Access layer 1 remembers it, so it is open. Place 1 is not
/// Access layer 2's: dimmed. In All, Place 1 is marked 1 (both of Access
/// layer 1's routes hold it), Places 2 and 3 are marked 1 2.
#[test]
fn each_layers_view_shows_its_checks_its_places_and_the_vaults_it_opens() {
    let mut app = two_layers_worked();
    let text = app.backup.as_ref().unwrap().answers.to_text();
    // All: the board as it was, each place marked with its layers.
    let drawn = app.board_places_drawn();
    let marks: Vec<Vec<usize>> = drawn.iter().map(|d| d.2.clone()).collect();
    assert_eq!(marks, vec![vec![1], vec![1, 2], vec![1, 2]]);
    assert!(drawn.iter().all(|d| !d.0 && !d.1), "nothing dimmed in All");
    // All draws the vault sealed, its description inside and in the list.
    assert_eq!(drawn_with(&mut app, "Vault 1 · sealed", "Whole"), Some(2));

    view(&mut app, View::Layer(0));
    let l1 = "Access layer 1, route 1 · paper";
    assert_eq!(check(&mut app, l1, "Can spend"), "Yes");
    assert_eq!(check(&mut app, l1, "Places it needs"), "2 of its 3");
    assert_eq!(check(&mut app, l1, "Sees the balance"), "Yes");
    let r2 = "Access layer 1, route 2";
    assert_eq!(check(&mut app, r2, "Can spend"), "Yes");
    assert_eq!(check(&mut app, r2, "Places it needs"), "1 of its 1");
    assert_eq!(check(&mut app, r2, "Sees the balance"), "Yes");
    assert_eq!(check(&mut app, "Access layer 1", "Places it can lose"), "1");
    assert!(app.board_places_drawn().iter().all(|d| !d.0));
    // Vault 1 open, its contents drawn.
    assert_eq!(drawn_with(&mut app, "Vault 1 · open", "Whole"), Some(2));

    view(&mut app, View::Layer(1));
    let l2 = "Access layer 2";
    assert_eq!(check(&mut app, l2, "Can spend"), "Yes");
    assert_eq!(check(&mut app, l2, "Places it needs"), "2 of its 2");
    assert_eq!(check(&mut app, l2, "Places it can lose"), "0");
    assert_eq!(check(&mut app, l2, "Sees the balance"), "Yes");
    let dim: Vec<bool> = app.board_places_drawn().iter().map(|d| d.0).collect();
    assert_eq!(dim, vec![true, false, false]);
    // Vault 1 shut: the description it holds is not drawn, only the
    // list's.
    assert_eq!(drawn_with(&mut app, "Vault 1 · shut", "Whole"), Some(1));
    assert_eq!(drawn_with(&mut app, "Vault 1 · open", "Whole"), None);

    // A change of view changes nothing in the plan.
    view(&mut app, View::Stranger);
    view(&mut app, View::All);
    assert_eq!(app.backup.as_ref().unwrap().answers.to_text(), text);
}

/// The Stranger view on the same plan, by hand. One place found: each
/// gives one seed and a share of two of the three keys (Place 1 its
/// vault's stick too, shut with no password written): no spending, no
/// balance, so both checks say yes (the margins are 1); the place that
/// gives the most is Place 1, by its stick. Two places found: any two
/// give two seeds and two shares covering the keys, so both say no; the
/// two with Place 1 give five things to Places 2 and 3's four, and of
/// those Places 1 and 2 come first.
#[test]
fn the_stranger_view_tries_every_set_of_places_found_and_shows_the_worst() {
    let mut app = two_layers_worked();
    let text = app.backup.as_ref().unwrap().answers.to_text();
    view(&mut app, View::Stranger);
    assert_eq!(
        check(&mut app, "", "A stranger with any one place can spend"),
        "No"
    );
    assert_eq!(
        check(
            &mut app,
            "",
            "A stranger with any one place sees the balance"
        ),
        "No"
    );
    let drawn = app.board_places_drawn();
    let lit: Vec<bool> = drawn.iter().map(|d| d.1).collect();
    let dim: Vec<bool> = drawn.iter().map(|d| d.0).collect();
    assert_eq!(lit, vec![true, false, false]);
    assert_eq!(dim, vec![false, true, true]);
    assert_eq!(drawn_with(&mut app, "Vault 1 · shut", "Whole"), Some(1));

    app.press(Action::Board(BP::FoundMore));
    assert_eq!(
        check(&mut app, "", "A stranger with any 2 places can spend"),
        "Yes"
    );
    assert_eq!(
        check(
            &mut app,
            "",
            "A stranger with any 2 places sees the balance"
        ),
        "Yes"
    );
    let lit: Vec<bool> = app.board_places_drawn().iter().map(|d| d.1).collect();
    assert_eq!(lit, vec![true, true, false]);
    // No more places than the plan has; no fewer than one.
    for _ in 0..3 {
        app.press(Action::Board(BP::FoundMore));
    }
    assert!(has_drawn(
        &mut app,
        "A stranger with any 3 places can spend"
    ));
    for _ in 0..5 {
        app.press(Action::Board(BP::FoundFewer));
    }
    assert!(has_drawn(
        &mut app,
        "A stranger with any one place can spend"
    ));
    assert_eq!(app.backup.as_ref().unwrap().answers.to_text(), text);
}

fn has_drawn(app: &mut Faraday, want: &str) -> bool {
    frames_down(app).iter().any(|t| t.iter().any(|x| x == want))
}
