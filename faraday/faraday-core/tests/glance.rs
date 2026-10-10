//! The wallet at a glance (`docs/NEW-WALLET.md` §6): a loaded wallet's
//! card charts its keys and its backup, where each thing is and what each
//! place gives whoever finds it. A cosigner's key is shown and nothing of
//! this backup holds its seed; with no plan the card offers Back up; the
//! plan is still charted after a lock, and from the vault that keeps it
//! when the wallet loads from it on another power-on; on a small panel
//! the chart is a page of its own. The chart's layout reads either way: the wallet on top, or
//! the backup. An open vault shows each wallet it holds the other way up,
//! its places above its keys and its keys above the wallet (§7.2).
//!
//! Every node, and every line inside a backup node, opens a sheet of what
//! it is and what can be done with it (§9): opening one puts nothing For
//! the stick, and each action leads into the flow that does it, coming
//! back to the chart where that flow can.

use faraday_core::glance::{Backup, BackupNode, Direction, Glance, Press, layout};
use faraday_core::glance_sheet::{ChartAction as C, Step, Target};
use faraday_core::vaults::VaultAction as V;
use faraday_core::{
    Action, BStage, Faraday, Screen, Sheet, StorageCommand, StorageEvent, bstep, testkit,
};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

type Files = Vec<(String, Vec<u8>)>;

fn device(width: u16, height: u16, dpi: u16) -> Faraday {
    device_with(width, height, dpi, Vec::new())
}

fn device_with(width: u16, height: u16, dpi: u16, kept: Files) -> Faraday {
    device_full(width, height, dpi, Vec::new(), kept)
}

fn device_full(width: u16, height: u16, dpi: u16, inbox: Files, kept: Files) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept,
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(180);
    let _ = app.frame();
    app
}

fn desktop() -> Faraday {
    device(1366, 768, 160)
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

/// Test key 1 typed in and a one-key wallet over it. Returns its index.
fn one_key(app: &mut Faraday) -> usize {
    add_key(app, 0);
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&app.session.keys[0].master)
        .unwrap();
    app.session
        .add_wallet("Spending", &format!("wpkh({text}/<0;1>/*)"), "test")
        .unwrap()
}

/// Test keys 1 and 2 typed in and the 2-of-3 over the three test keys:
/// the third is a cosigner's. Returns its index.
fn two_of_three(app: &mut Faraday) -> usize {
    add_key(app, 0);
    add_key(app, 1);
    app.session
        .add_wallet("Savings", &testkit::savings(), "test")
        .unwrap()
}

/// The backup of wallet `w` planned with preset `preset` and its
/// checklist made; then the wallet's card.
fn planned(app: &mut Faraday, w: usize, preset: u8) {
    app.press(Action::Backup(w));
    app.press(Action::BPreset(preset));
    app.press(Action::BChecklist);
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
}

fn chart(app: &Faraday, w: usize) -> Glance {
    faraday_core::glance::of(app, w).expect("the wallet has a chart")
}

fn nodes(g: &Glance) -> Vec<BackupNode> {
    match &g.backup {
        Backup::Plan { nodes, .. } => nodes.clone(),
        other => panic!("no plan on the chart: {other:?}"),
    }
}

fn node<'a>(nodes: &'a [BackupNode], name: &str) -> &'a BackupNode {
    nodes
        .iter()
        .find(|n| n.name == name)
        .unwrap_or_else(|| panic!("no {name} on the chart"))
}

#[test]
fn a_single_key_with_paper_and_vault_shows_two_places_and_the_vault() {
    let mut app = desktop();
    let w = one_key(&mut app);
    planned(&mut app, w, 1);
    let g = chart(&app, w);
    assert_eq!(g.keys.len(), 1);
    assert_eq!(g.keys[0].state, "Can sign here");
    let nodes = nodes(&g);
    let names: Vec<&str> = nodes.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, ["Place 1", "Place 2", "Vault 1"]);
    let one = node(&nodes, "Place 1");
    assert!(one.holds.contains(&"Key 1 words".to_string()), "{one:?}");
    assert!(!one.holds.iter().any(|h| h.contains("stick")), "{one:?}");
    let two = node(&nodes, "Place 2");
    assert!(two.holds.contains(&"Vault 1 stick".to_string()), "{two:?}");
    let vault = node(&nodes, "Vault 1");
    assert!(vault.holds.contains(&"Key 1 seed".to_string()), "{vault:?}");
    assert_eq!(vault.stick.as_deref(), Some("Stick at Place 2"));
    assert_eq!(vault.alone.as_deref(), Some("Sealed"));
    // The key's lines go to the places with its words and to its vault.
    assert!(one.keys == [0] && vault.keys == [0], "{nodes:?}");
    let texts = app.drawn_texts();
    for want in [
        "At a glance",
        "Place 1",
        "Place 2",
        "Vault 1",
        "Key 1 words",
    ] {
        assert!(texts.iter().any(|t| t == want), "{want} is not on the card");
    }
}

#[test]
fn a_cosigners_key_is_xpub_only_and_no_backup_node_holds_its_seed() {
    let mut app = desktop();
    let w = two_of_three(&mut app);
    planned(&mut app, w, 1);
    let g = chart(&app, w);
    let states: Vec<&str> = g.keys.iter().map(|k| k.state.as_str()).collect();
    assert_eq!(
        states,
        ["Can sign here", "Can sign here", "Cosigner · xpub only"]
    );
    let nodes = nodes(&g);
    for n in &nodes {
        assert!(!n.keys.contains(&2), "{} has a line from key 3", n.name);
        assert!(
            !n.holds.iter().any(|h| h.starts_with("Key 3")),
            "{} holds key 3's seed: {:?}",
            n.name,
            n.holds
        );
    }
    // A vault per key here, each stick at a place without that key's
    // words.
    assert_eq!(
        node(&nodes, "Vault 1").stick.as_deref(),
        Some("Stick at Place 3")
    );
    assert_eq!(
        node(&nodes, "Vault 2").stick.as_deref(),
        Some("Stick at Place 2")
    );
}

#[test]
fn a_place_with_the_whole_sheet_sees_the_balance() {
    let mut app = desktop();
    let w = two_of_three(&mut app);
    planned(&mut app, w, 0);
    let nodes = nodes(&chart(&app, w));
    // The cosigner's place keeps the sheet alone.
    let three = node(&nodes, "Place 3");
    assert_eq!(three.holds, ["Wallet sheet"]);
    assert_eq!(three.alone.as_deref(), Some("Sees the balance"));
    // One key's words and the sheet spend nothing in a 2-of-3.
    assert_eq!(
        node(&nodes, "Place 1").alone.as_deref(),
        Some("Sees the balance")
    );
    assert!(app.drawn_texts().iter().any(|t| t == "Sees the balance"));
}

#[test]
fn with_no_plan_the_chart_says_so_and_back_up_opens_the_backup() {
    let mut app = desktop();
    let w = one_key(&mut app);
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(chart(&app, w).backup, Backup::None);
    assert!(app.drawn_texts().iter().any(|t| t == "No backup plan"));
    assert!(app.offers(Action::Backup(w)));
    app.press(Action::Backup(w));
    assert_eq!(app.screen, Screen::Backup);
}

/// The names For the stick.
fn for_the_stick(app: &Faraday) -> Vec<String> {
    app.outbox.iter().map(|i| i.name.clone()).collect()
}

fn open(app: &mut Faraday, press: Press, t: Target) {
    app.press(Action::Chart(C::Open(press, t)));
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Chart), "{t:?} opened no sheet");
}

fn shown(app: &mut Faraday, want: &str) -> bool {
    app.drawn_texts().iter().any(|t| t == want)
}

#[test]
fn pressing_a_place_opens_its_sheet_and_puts_nothing_for_the_stick() {
    let mut app = desktop();
    let w = one_key(&mut app);
    planned(&mut app, w, 1);
    // A backup of it opened again starts on the presets; the card still
    // charts the plan this power-on made.
    app.press(Action::Backup(w));
    assert_eq!(app.backup.as_ref().map(|b| b.stage), Some(BStage::Plan));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(nodes(&chart(&app, w)).len(), 3, "the plan is still charted");
    let before = for_the_stick(&app);
    let press = Press::Loaded(w);
    assert!(
        app.offers(Action::Chart(C::Open(press, Target::Node(0)))),
        "the places are not pressable"
    );
    open(&mut app, press, Target::Node(0));
    for want in ["Place 1", "Key 1 words", "What goes in its envelope"] {
        assert!(shown(&mut app, want), "no {want} on the place's sheet");
    }
    assert_eq!(for_the_stick(&app), before, "opening the sheet made files");
    // Its envelope: the checklist at that item, from the plan the chart
    // shows, and still nothing made.
    let envelope = Action::Chart(C::Do(press, Step::Item(bstep::ENVELOPE)));
    assert!(app.offers(envelope));
    app.press(envelope);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Backup);
    let b = app.backup.as_ref().unwrap();
    assert_eq!(b.stage, BStage::Checklist);
    assert_eq!(b.open, Some(bstep::ENVELOPE));
    assert!(b.answers.seeds[faraday_core::plan::seeds::VAULT]);
    assert_eq!(for_the_stick(&app), before, "the checklist made files");
    // Its way back is the wallet's card, and the chart.
    let _ = app.frame();
    assert!(app.offers(Action::OpenWallet(w)));
}

#[test]
fn a_line_inside_a_place_opens_its_own_sheet() {
    let mut app = desktop();
    let w = two_of_three(&mut app);
    planned(&mut app, w, 0);
    let press = Press::Loaded(w);
    let at = nodes(&chart(&app, w))
        .iter()
        .position(|n| n.name == "Place 3")
        .unwrap() as u8;
    let line = Action::Chart(C::Open(press, Target::Line(at, 0)));
    assert!(app.offers(line), "the line is not pressable on the chart");
    open(&mut app, press, Target::Line(at, 0));
    assert!(shown(&mut app, "Wallet sheet"));
    assert!(shown(&mut app, "Print it again"));
    // Printed again For the stick, and the chart is where it was.
    app.press(Action::PublicRemove(w, 3));
    assert!(
        !for_the_stick(&app)
            .iter()
            .any(|n| n.ends_with("-backup.pdf"))
    );
    open(&mut app, press, Target::Line(at, 0));
    let print = Action::Chart(C::Do(press, Step::Out(3)));
    app.press(print);
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.sheet, None);
    assert!(
        for_the_stick(&app)
            .iter()
            .any(|n| n.ends_with("-backup.pdf")),
        "{:?}",
        for_the_stick(&app)
    );
}

#[test]
fn a_key_held_here_shows_its_xpub_and_where_its_seed_is() {
    let mut app = desktop();
    let w = one_key(&mut app);
    planned(&mut app, w, 1);
    let press = Press::Loaded(w);
    let fp = app.session.keys[0].master.fingerprint().0;
    open(&mut app, press, Target::Key(0));
    assert!(shown(&mut app, "Make another backup of this seed"));
    assert!(app.offers(Action::Chart(C::Do(press, Step::Scan(fp)))));
    assert!(app.offers(Action::Chart(C::Do(press, Step::Item(bstep::COPY)))));
    // The xpub's code over the card; closed, the chart again.
    app.press(Action::Chart(C::Do(press, Step::KeyQr(0))));
    assert_eq!(app.sheet, Some(Sheet::Qr));
    app.press(Action::Cancel);
    assert_eq!(app.screen, Screen::Wallets);
    // Where it is: its lines drawn bold, the sheet closed.
    open(&mut app, press, Target::Key(0));
    app.press(Action::Chart(C::Where(press, 0)));
    assert_eq!(app.sheet, None);
    assert_eq!(chart(&app, w).focus, Some(0));
    // Scanning a copy keeps the chart under the camera.
    open(&mut app, press, Target::Key(0));
    app.press(Action::Chart(C::Do(press, Step::Scan(fp))));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    app.press(Action::Cancel);
    assert_eq!(app.screen, Screen::Wallets);
}

#[test]
fn a_cosigners_key_offers_nothing_secret() {
    let mut app = desktop();
    let w = two_of_three(&mut app);
    planned(&mut app, w, 1);
    let press = Press::Loaded(w);
    open(&mut app, press, Target::Key(2));
    assert!(shown(&mut app, "Cosigner · xpub only"));
    for not in ["Show xpub QR", "Scan my copy", "By hand"] {
        assert!(!shown(&mut app, not), "{not} offered for a cosigner's key");
    }
}

#[test]
fn on_a_small_panel_the_chart_is_its_own_page() {
    let mut app = device(480, 640, 286);
    let w = two_of_three(&mut app);
    planned(&mut app, w, 1);
    let open = Action::Glance(true);
    let mut offered = app.offers(open);
    for _ in 0..10 {
        if offered {
            break;
        }
        app.event(Event::Scroll {
            x: 240,
            y: 320,
            dy: 200,
        });
        app.settle();
        let _ = app.frame();
        offered = app.offers(open);
    }
    assert!(offered, "the card has no At a glance row");
    app.press(open);
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "At a glance"), "{texts:?}");
    // A place names the keys a vault's stick opens.
    let nodes = nodes(&chart(&app, w));
    assert_eq!(
        node(&nodes, "Place 2").keys_held,
        "Key 2 words · Wallet sheet · Key 2 in the vault"
    );
    // A node pressed: its sheet is the page, and Back returns to the
    // chart.
    let press = Press::Loaded(w);
    app.press(Action::Chart(C::Open(press, Target::Node(0))));
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Chart));
    assert!(shown(&mut app, "Back"), "the sheet has no Back");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert!(app.glance, "Back left the chart");
    let _ = app.frame();
    assert!(app.offers(Action::Glance(false)), "no way back to the card");
    app.press(Action::Glance(false));
    assert!(!app.glance);
    assert_eq!(app.screen, Screen::Wallets);
}

/// What the lock saved: For the stick, and the kept state.
fn saved_at_lock(app: &mut Faraday) -> (Files, Files) {
    app.press(Action::Lock);
    let mut last = (Vec::new(), Vec::new());
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, kept, .. } = c {
            last = (outbox, kept);
        }
    }
    last
}

fn kept_at_lock(app: &mut Faraday) -> Files {
    saved_at_lock(app).1
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

#[test]
fn after_a_lock_the_wallet_loaded_again_still_charts_its_plan() {
    let mut app = desktop();
    let w = one_key(&mut app);
    planned(&mut app, w, 1);
    let before = nodes(&chart(&app, w));
    let kept = kept_at_lock(&mut app);
    let mut app = device_with(1366, 768, 160, kept);
    let w = one_key(&mut app);
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    let after = nodes(&chart(&app, w));
    let names = |n: &[BackupNode]| n.iter().map(|n| n.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&after), names(&before));
    assert_eq!(
        node(&after, "Vault 1").holds,
        node(&before, "Vault 1").holds
    );
}

#[test]
fn a_wallet_loaded_from_its_vault_on_another_power_on_charts_the_vaults_plan() {
    let mut app = desktop();
    let w = one_key(&mut app);
    // A vault made and open, the plan made with it, the seed and the
    // wallet saved into it.
    for i in 0..8u8 {
        app.event(Event::Entropy(osk_shell_api::EntropyBytes::new(
            [0x40 + i; 32],
        )));
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(&mut app, "test phrase");
    }
    app.press(Action::Vault(V::CGo));
    settle(&mut app, |a| a.screen == Screen::Unlock);
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| !a.vaults.open.is_empty());
    assert_eq!(app.vaults.open.len(), 1, "the vault did not open");
    planned(&mut app, w, 1);
    app.press(Action::Backup(w));
    app.press(Action::BChecklist);
    app.press(Action::BVault(false));
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::AddKind(1)));
    app.press(Action::Vault(V::SaveWallet(0)));
    let before = nodes(&chart(&app, w));

    // Another power-on: nothing kept, the vault file read off the stick,
    // unlocked, and its wallet loaded with its seed.
    let (outbox, _) = saved_at_lock(&mut app);
    let vault: Files = outbox
        .into_iter()
        .filter(|(n, _)| n.ends_with(".ofv"))
        .collect();
    assert_eq!(vault.len(), 1, "the vault was not put For the stick");
    let mut app = device_full(1366, 768, 160, vault, Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| !a.vaults.open.is_empty());
    assert_eq!(app.vaults.open.len(), 1, "the vault did not unlock");
    app.press(Action::Vault(V::LoadWithKeys(0)));
    assert_eq!(app.session.wallets.len(), 1, "the wallet did not load");
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let after = nodes(&chart(&app, 0));
    let names = |n: &[BackupNode]| n.iter().map(|n| n.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&after), ["Place 1", "Place 2", "Vault 1"]);
    assert_eq!(names(&after), names(&before));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Vault 1"), "{texts:?}");
}

#[test]
fn the_layout_reads_wallet_first_or_backup_first_and_no_line_crosses_a_node() {
    // A 2-of-3 with two keys here and five backup nodes, in a column too
    // narrow for the five on one line.
    let sources = vec![
        ((0, 0), vec![(1, 0), (1, 1), (1, 2)]),
        ((1, 0), vec![(2, 0), (2, 3)]),
        ((1, 1), vec![(2, 1), (2, 4)]),
    ];
    let height = |_: usize, _: usize, _: f32| 60.0;
    for (direction, order) in [
        (Direction::WalletFirst, [0, 1, 2]),
        (Direction::BackupFirst, [2, 1, 0]),
    ] {
        let l = layout(
            direction,
            0.0,
            0.0,
            400.0,
            200.0,
            [1, 3, 5],
            &height,
            &sources,
        );
        let top = |r: usize| l.nodes[r].iter().map(|p| p.y).fold(f32::MAX, f32::min);
        let foot = |r: usize| l.nodes[r].iter().map(|p| p.y + p.h).fold(0.0, f32::max);
        assert!(
            foot(order[0]) < top(order[1]) && foot(order[1]) < top(order[2]),
            "{direction:?}: rows out of order"
        );
        assert_eq!(l.paths.len(), 7, "{direction:?}: a line per pair joined");
        let all: Vec<_> = l.nodes.iter().flatten().copied().collect();
        for (s, pts) in &l.paths {
            for seg in pts.windows(2) {
                let ((x0, y0), (x1, y1)) = (seg[0], seg[1]);
                assert!(x0 == x1 || y0 == y1, "{direction:?}: line {s} runs aslant");
                for p in &all {
                    let across = x0.min(x1) < p.x + p.w && x0.max(x1) > p.x;
                    let down = y0.min(y1) < p.y + p.h && y0.max(y1) > p.y;
                    let inside_x = x0 > p.x && x0 < p.x + p.w;
                    let inside_y = y0 > p.y && y0 < p.y + p.h;
                    let crosses = if y0 == y1 {
                        across && inside_y
                    } else {
                        down && inside_x
                    };
                    assert!(!crosses, "{direction:?}: line {s} crosses a node at {p:?}");
                }
            }
        }
    }
}

/// A vault made and unlocked; with `plan`, the backup of single-key
/// wallet `w` planned with the Paper and vault preset while it is open,
/// its seed saved into it; then the wallet saved into it. Then the
/// vault's wallets, the first chosen.
fn vault_holding(app: &mut Faraday, w: usize, plan: bool) {
    for i in 0..8u8 {
        app.event(Event::Entropy(osk_shell_api::EntropyBytes::new(
            [0x40 + i; 32],
        )));
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(app, "test phrase");
    }
    app.press(Action::Vault(V::CGo));
    settle(app, |a| a.screen == Screen::Unlock);
    type_text(app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults.open.is_empty());
    assert_eq!(app.vaults.open.len(), 1, "the vault did not open");
    if plan {
        planned(app, w, 1);
        app.press(Action::Backup(w));
        app.press(Action::BChecklist);
        app.press(Action::BVault(false));
    }
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::AddKind(1)));
    app.press(Action::Vault(V::SaveWallet(w)));
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(1)));
    app.press(Action::Vault(V::Item(0)));
    let _ = app.frame();
}

/// The wallet record the open vault's view charts.
fn vault_wallet(app: &Faraday) -> usize {
    app.vault_selected_index().expect("a wallet is chosen")
}

#[test]
fn an_open_vault_shows_its_wallet_from_the_places_down_to_the_wallet() {
    let mut app = desktop();
    let w = one_key(&mut app);
    vault_holding(&mut app, w, true);
    let r = vault_wallet(&app);
    let texts = app.drawn_texts();
    for want in ["Place 1", "Place 2", "Vault 1", "Spending"] {
        assert!(texts.iter().any(|t| t == want), "no {want}: {texts:?}");
    }
    let press = Press::Vault(0, r);
    let at = |t: Target| {
        app.hit_box(Action::Chart(C::Open(press, t)))
            .unwrap_or_else(|| panic!("no {t:?} on the chart"))
    };
    let (place, key, wallet) = (at(Target::Node(0)), at(Target::Key(0)), at(Target::Wallet));
    assert!(place.1 + place.3 <= key.1, "a place is not above the key");
    assert!(key.1 + key.3 <= wallet.1, "the key is not above the wallet");
    // The wallet pressed: its sheet, which opens its card.
    open(&mut app, press, Target::Wallet);
    let card = Action::Vault(V::OpenWalletOf(0, r));
    assert!(app.offers(card));
    app.press(card);
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.wallet, w);
    // A place pressed: its sheet; its envelope opens the backup's
    // checklist, whose way back is the vault.
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    open(&mut app, press, Target::Node(0));
    app.press(Action::Chart(C::Do(press, Step::Item(bstep::ENVELOPE))));
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(
        app.backup.as_ref().map(|b| b.stage),
        Some(BStage::Checklist)
    );
    let _ = app.frame();
    let back = Action::Nav(Screen::VaultContents);
    assert!(
        app.offers(back),
        "the checklist does not lead back to the vault"
    );
    app.press(back);
    let _ = app.frame();
    // Renamed in the open vault: the plan's place names, typing.
    open(&mut app, press, Target::Node(0));
    app.press(Action::Chart(C::Do(press, Step::RenamePlace(0))));
    assert_eq!(app.screen, Screen::Backup);
    let b = app.backup.as_ref().unwrap();
    assert_eq!((b.stage, b.naming), (BStage::Plan, Some(0)));
}

#[test]
fn a_wallet_with_no_plan_in_the_vault_shows_its_wallet_and_keys_only() {
    let mut app = desktop();
    let w = one_key(&mut app);
    vault_holding(&mut app, w, false);
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "No backup plan in this vault"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t == "Spending"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Place 1"), "{texts:?}");
}
