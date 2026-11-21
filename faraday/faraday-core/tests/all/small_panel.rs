//! A Raspberry Pi's panel, 480 × 640 at 286 dpi: no sidebar, Home as the
//! menu, every flow a page per step, a keyboard docked at the foot. What
//! a person can reach there by touch alone, scrolling where a page is
//! longer than the panel.

use faraday_core::keygen::{Group, Way};
use faraday_core::testkit::Plain;
use faraday_core::vault::VaultAction as V;
use faraday_core::vault::vstep;
use faraday_core::{Action, ChoicesPress, Faraday, Opening, OskPress, Screen, Sheet};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

const W: u16 = 480;
const H: u16 = 640;

thread_local! {
    /// The size of the panel the test drew on, pixels.
    static SIZE: std::cell::Cell<(u16, u16)> = const { std::cell::Cell::new((W, H)) };
}

/// The app as the Pi shell starts it, drawn once.
fn panel() -> Faraday {
    SIZE.with(|s| s.set((W, H)));
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: W,
        height: H,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

/// Where `action` can be pressed on the panel as it is drawn now.
fn on_panel(app: &mut Faraday, action: Action) -> Option<(u16, u16)> {
    let _ = app.frame();
    let (w, h) = SIZE.with(std::cell::Cell::get);
    app.where_offered(action).filter(|&(x, y)| x < w && y < h)
}

/// Scrolls the page down until `action` is on the panel. Panics when it
/// never comes into view.
fn find(app: &mut Faraday, action: Action) -> (u16, u16) {
    for _ in 0..80 {
        if let Some(at) = on_panel(app, action) {
            return at;
        }
        app.event(Event::Scroll {
            x: W / 2,
            y: H / 2,
            dy: 60,
        });
    }
    panic!("{action:?} is never on the panel");
}

/// Your choices, from the bar's ⋮ (the Pi's panel has no button for it
/// on the bar): the ⋮ is searched over every Overflow it can hold.
fn open_choices(app: &mut Faraday) {
    use faraday_core::nav::{BarTool, MenuPress, Overflow};
    let _ = app.frame();
    let open = (0u8..64)
        .map(|bits| {
            Action::NavMenu(MenuPress::Open(Some(Overflow {
                own: bits & 1 != 0,
                choices: bits & 16 != 0,
                help: bits & 2 != 0,
                keyboard: bits & 4 != 0,
                keyboard_up: bits & 32 != 0,
                eye: bits & 8 != 0,
            })))
        })
        .find(|a| app.offers(*a))
        .expect("the bar's ⋮");
    app.press(open);
    let _ = app.frame();
    app.press(Action::NavMenu(MenuPress::Tool(BarTool::Choices)));
    let _ = app.frame();
}

/// A finger on `action`, wherever it has scrolled to.
fn tap(app: &mut Faraday, action: Action) {
    let (x, y) = find(app, action);
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    // The frame that shows the press, and the tick after it, on which a
    // press that stretches a key does its work (`docs/SPEED.md` §9).
    let _ = app.frame();
    // Then the frames of a step's fade, as a shell draws them: a step
    // that changed shows as the card.
    for _ in 0..40 {
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        let _ = app.frame();
    }
}

#[test]
fn homes_grid_offers_guide_and_not_add_a_key() {
    let mut app = panel();
    find(&mut app, Action::Guide);
    assert!(
        on_panel(&mut app, Action::Entry(None)).is_none(),
        "Add a key is reached from Wallets now, not Home"
    );
    tap(&mut app, Action::Guide);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Guide));
}

#[test]
fn home_offers_every_place_without_a_sidebar() {
    let mut app = panel();
    assert_eq!(app.screen, Screen::Home);
    for place in [
        Screen::WalletsStart,
        Screen::Vaults,
        Screen::Files,
        Screen::Catalog,
        Screen::Settings,
    ] {
        find(&mut app, Action::Nav(place));
    }
    tap(&mut app, Action::Nav(Screen::Settings));
    assert_eq!(app.screen, Screen::Settings);
    tap(&mut app, Action::Nav(Screen::Home));
    assert_eq!(app.screen, Screen::Home, "the bar goes back to Home");
}

/// Spend left the small Home grid: the
/// Wallets empty state and the guide sheet reach it instead.
#[test]
fn homes_grid_has_no_spend_tile() {
    let mut app = panel();
    assert!(on_panel(&mut app, Action::Nav(Screen::SpendBackup)).is_none());
}

#[test]
fn a_wallet_is_made_by_touch_from_a_key_rolled_on_dice() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    // Create opens on Kind with Single key ticked and Words answering
    // Backup: pressing each takes it and moves on to Keys, with nothing
    // loaded its New key for slot 1.
    tap(&mut app, Action::CKind(0));
    tap(
        &mut app,
        Action::CBackupForm(faraday_core::create::BackupForm::Words.index()),
    );
    assert_eq!(app.screen, Screen::Create);
    tap(&mut app, Action::KeyOtherWays(0));
    assert_eq!(app.screen, Screen::KeyGen);
    // The key's cards, as Other ways opens them, has Start answered by the Backup
    // card and opens on Length; pressing a count takes it and opens
    // Randomness (F103).
    assert_eq!(
        app.keygen().and_then(|k| k.open),
        Some(faraday_core::keygen::kstep::LENGTH),
        "Start is answered, Length opens"
    );
    tap(&mut app, Action::KWords(12));
    assert_eq!(
        app.keygen().and_then(|k| k.open),
        Some(faraday_core::keygen::kstep::SOURCE),
        "Length's Continue opens Randomness"
    );
    // Dice hashed into the words: its group opened first.
    tap(&mut app, Action::KGroup(Group::Computed as u8));
    tap(&mut app, Action::KWay(Way::DiceHashed.index()));
    // The panel starts on the dice's buttons: no keyboard needed.
    for i in 0..99u8 {
        tap(&mut app, Action::KRoll(i % 6 + 1));
    }
    // The entry card's Continue opens Key; Lock in adds the key and,
    // a single key being the whole wallet, makes it, and returns to
    // Create on Check, Back up wallet its foot, by itself (F103).
    tap(&mut app, Action::KNext);
    tap(&mut app, Action::KLock);
    assert_eq!(app.screen, Screen::Create, "the key fills its slot");
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::CHECK)
    );
    find(&mut app, Action::CBackup);
    let made = app.create().and_then(|c| c.built);
    assert!(made.is_some(), "Lock in made the wallet");
}

#[test]
fn a_passphrase_field_brings_the_keyboard_and_stays_above_it() {
    let mut app = panel();
    // With no vault file in Files, Vaults opens New vault itself.
    tap(&mut app, Action::Nav(Screen::Vaults));
    assert_eq!(app.screen, Screen::CreateVault);
    // It opens on Name and passphrase at its top, nothing focused and
    // the keyboard down; a tap on a field brings it up.
    let key = Action::Osk(OskPress::Char('a'));
    assert!(
        on_panel(&mut app, key).is_none(),
        "the page opens with the keyboard down"
    );
    tap(&mut app, Action::Vault(V::CFocus(0, false)));
    assert!(on_panel(&mut app, key).is_some(), "a tap brings it up");
    // Put away, then another field pressed: the keyboard comes back, and
    // the field it covered moves up above it.
    tap(&mut app, Action::Osk(OskPress::Hide));
    assert!(on_panel(&mut app, key).is_none());
    let field = Action::Vault(V::CFocus(0, true));
    tap(&mut app, field);
    let (_, key_y) = on_panel(&mut app, key).expect("the keyboard is up");
    let (_, field_y) = on_panel(&mut app, field).expect("the field is still on the panel");
    assert!(
        field_y < key_y,
        "the field is above the keyboard, not under it"
    );
    tap(&mut app, key);
    tap(&mut app, key);
    let typed = app.vaults().create.as_ref().unwrap().phrases[0]
        .1
        .text
        .clone();
    assert_eq!(typed.as_str(), "aa", "the keys type into the field");
}

/// The app on a phone's panel, 5 inches at 9:16, as `faraday --panel 5`
/// shows it on a laptop of 277 pixels an inch, drawn once.
fn phone() -> Faraday {
    SIZE.with(|s| s.set((678, 1206)));
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 678,
        height: 1206,
        dpi: 277,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

/// Where `action` is on the first screenful, with no scrolling.
fn in_view(app: &mut Faraday, h: u16, action: Action) -> bool {
    let _ = app.frame();
    app.where_offered(action).is_some_and(|(_, y)| y < h)
}

/// Each step of Create a vault and Make a new wallet, opened on its own:
/// its forward action is on the panel without scrolling.
fn forward_in_view(mut app: Faraday, h: u16) {
    // With no vault file in Files, Vaults opens New vault itself.
    tap(&mut app, Action::Nav(Screen::Vaults));
    // The size, opened on its own; then Customise's two cards in its
    // place.
    // No row of Your choices is on a one-column layout until its bar's
    // button and sheet: the step is opened by its press.
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    // A size, a cost or a space pressed moves on (F103): the first row
    // is the way on there.
    let next = Action::Vault(V::CSizeRow(0));
    assert!(in_view(&mut app, h, next), "Create a vault, the size");
    tap(&mut app, Action::Vault(V::CCustomise));
    for next in [V::CNext(vstep::WHERE), V::CPreset(0)] {
        let next = Action::Vault(next);
        assert!(in_view(&mut app, h, next), "Create a vault, {next:?}");
        tap(&mut app, next);
    }
    // The last step opens on its first passphrase with the keyboard up,
    // which leaves no room for the bar: put away, the bar is back.
    if on_panel(&mut app, Action::Osk(OskPress::Hide)).is_some() {
        tap(&mut app, Action::Osk(OskPress::Hide));
    }
    assert!(
        in_view(&mut app, h, Action::Vault(V::CGo)),
        "Create vault, on the last step"
    );
    // Back, through the steps to Home, where Vaults was pressed.
    for _ in 0..8 {
        if app.screen == Screen::CreateVault {
            tap(&mut app, Action::Nav(Screen::Home));
        }
    }
    assert_eq!(app.screen, Screen::Home);
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::CreateWallet);
    // Create opens on Kind. A row is Kind's forward action: Single key
    // (0) and Multisig (4), shown before More kinds.
    for kind in [0, 4] {
        assert!(
            in_view(&mut app, h, Action::CKind(kind)),
            "Make a new wallet, Kind, row {kind}"
        );
    }
    tap(&mut app, Action::CKind(0));
    // Backup shows every way at once.
    for form in faraday_core::create::BackupForm::offered() {
        assert!(
            in_view(&mut app, h, Action::CBackupForm(form.index())),
            "Make a new wallet, Backup, {form:?}"
        );
    }
}

/// Create opens on Kind; Single key opens Backup, whose Words opens Use
/// vaults, whose No opens
/// Keys (a key loaded, so the card has a choice), where Kind is a row of
/// Your choices.
#[test]
fn kind_is_offered_on_the_keys_page_and_opens() {
    let mut app = panel();
    crate::common::add_key(&mut app, "bacon");
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    app.press(Action::CreateWallet);
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::KIND),
        "Create opens on Kind"
    );
    tap(&mut app, Action::CKind(0));
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::BACKUP_FORM),
        "Single key opens Backup"
    );
    tap(
        &mut app,
        Action::CBackupForm(faraday_core::create::BackupForm::Words.index()),
    );
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::KEYS),
        "Words opens Keys"
    );
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::KEYS),
        "Words opens Keys"
    );
    // Kind is reached from Your choices, on the bar's button.
    open_choices(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Choices));
    tap(
        &mut app,
        Action::Choices(ChoicesPress::Row(faraday_core::cstep::KIND)),
    );
    assert_eq!(app.sheet, None);
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::KIND),
        "its row opens Kind"
    );
}

/// The steps reached are Your choices, from which any opens.
#[test]
fn the_page_title_shows_the_steps() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::CreateWallet);
    tap(&mut app, Action::CKind(0));
    tap(
        &mut app,
        Action::CBackupForm(faraday_core::create::BackupForm::Words.index()),
    );
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::KEYS)
    );
    // Only the open step is on the page; the steps reached are rows of
    // Your choices.
    let kind = Action::CStep(faraday_core::cstep::KIND);
    assert!(on_panel(&mut app, kind).is_none(), "no list on the page");
    open_choices(&mut app);
    tap(
        &mut app,
        Action::Choices(ChoicesPress::Row(faraday_core::cstep::KIND)),
    );
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::KIND)
    );
}

/// Make a new wallet states its step's number alone until the whole count
/// is known, so a total is never shown and then changed.
#[test]
fn make_a_new_wallet_counts_its_steps_only_once_it_knows_how_many() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::CreateWallet);
    tap(&mut app, Action::CKind(0));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Step 2"),
        "{:?}",
        texts.iter().map(|t| t.to_string()).collect::<Vec<_>>()
    );
    assert!(
        !texts
            .iter()
            .any(|t| t.starts_with("Step ") && t.contains(" of ")),
        "{:?}",
        texts.iter().map(|t| t.to_string()).collect::<Vec<_>>()
    );
    tap(
        &mut app,
        Action::CBackupForm(faraday_core::create::BackupForm::Words.index()),
    );
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::KEYS)
    );
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Step 3 of 4"),
        "{:?}",
        texts.iter().map(|t| t.to_string()).collect::<Vec<_>>()
    );
}

#[test]
fn every_step_s_forward_action_is_on_a_2_8_inch_panel_without_scrolling() {
    forward_in_view(panel(), H);
}

/// Two boxes overlap.
fn overlaps(a: (u16, u16, u16, u16), b: (u16, u16, u16, u16)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
}

/// Scan floats over the foot of Home; no tile's hit area is under it.
#[test]
fn scan_never_covers_a_tile() {
    let mut app = panel();
    let _ = app.frame();
    let scan = app.hit_box(Action::Scan).expect("Scan is offered");
    for action in [
        Action::Nav(Screen::WalletsStart),
        Action::Nav(Screen::Vaults),
        Action::Nav(Screen::Files),
        Action::Guide,
        Action::Nav(Screen::Catalog),
        Action::Nav(Screen::Settings),
    ] {
        if let Some(tile) = app.hit_box(action) {
            assert!(
                !overlaps(scan, tile),
                "{action:?}'s tile is under Scan: {tile:?} vs {scan:?}"
            );
        }
    }
}

/// A flow's page title is the card's name alone, never a count next to
/// it that could be read as one of the card's own values; the step
/// count is its own muted "Step N of M" text beside the title
/// (`DESIGN.md` §4.1).
#[test]
fn a_flow_s_page_title_carries_no_count() {
    let mut app = panel();
    // With no vault file in Files, Vaults opens New vault itself.
    tap(&mut app, Action::Nav(Screen::Vaults));
    let texts = app.drawn_texts();
    // Create a vault is two cards, and opens on the second: the size
    // has its default. The title wraps onto
    // its own line or lines, with no count joined onto it.
    assert!(
        texts
            .iter()
            .any(|t| t == "Name and passphrase" || t == "Name and" || t == "passphrase"),
        "the title's words are drawn on their own: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.contains('·') && t.contains("of")),
        "the title still carries a count next to it: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "Step 2 of 2"),
        "the step count is drawn as its own, muted text: {texts:?}"
    );
}

/// The small Home leads with a tile per flow in progress, its name and
/// open step, opening it as it was left (`docs/FLOWS.md`, Flows in
/// progress).
#[test]
fn the_small_homes_first_tile_opens_the_flow_in_progress() {
    use faraday_core::in_progress::Going;
    let mut app = panel();
    let row = Action::Resume(Going::Create);
    assert!(on_panel(&mut app, row).is_none(), "nothing in progress");
    tap(&mut app, Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    tap(&mut app, Action::CKind(0));
    // The way back opens Kind first (F104), then leaves for Home.
    tap(&mut app, Action::Nav(Screen::Home));
    assert_eq!(app.screen, Screen::Create);
    let open = app.create().unwrap().open;
    tap(&mut app, Action::Nav(Screen::Home));
    assert_eq!(app.screen, Screen::Home);
    let (_, at) = on_panel(&mut app, row).expect("at the head of Home, in view");
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Make a new wallet"),
        "its name: {texts:?}"
    );
    let (_, tiles) = app.where_offered(Action::Nav(Screen::Vaults)).unwrap();
    assert!(at < tiles, "before the other tiles");
    tap(&mut app, row);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create().unwrap().open, open, "as it was left");
}

/// A lead that would start the flow in progress over is not drawn beside
/// its tile: Make a new wallet is on the small Home once, the tile.
#[test]
fn the_small_home_offers_a_flow_in_progress_once_its_tile() {
    use faraday_core::in_progress::Going;
    let mut app = panel();
    tap(&mut app, Action::CreateWallet);
    tap(&mut app, Action::CKind(0));
    tap(&mut app, Action::Nav(Screen::Home));
    tap(&mut app, Action::Nav(Screen::Home));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert_eq!(
        texts.iter().filter(|t| *t == "Make a new wallet").count(),
        1,
        "{texts:?}"
    );
    assert!(app.offers(Action::Resume(Going::Create)), "the tile");
    assert!(
        !app.offers(Action::CreateWallet),
        "no lead that starts it over"
    );
}

#[test]
fn every_step_s_forward_action_is_on_a_phone_without_scrolling() {
    forward_in_view(phone(), 1206);
}

#[test]
fn make_a_new_vault_offers_your_choices_as_a_sheet() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::Vaults));
    assert_eq!(app.screen, Screen::CreateVault);
    let open = |app: &Faraday| app.vaults().create.as_ref().and_then(|c| c.open);
    app.press(Action::Vault(V::CStep(vstep::PHRASES)));
    open_choices(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Choices));
    let _ = app.frame();
    let shown: Vec<String> = app
        .drawn_text_boxes()
        .plain()
        .into_iter()
        .map(|(t, _)| t)
        .collect();
    for row in ["Opens on", "Unlock cost", "Name and passphrase"] {
        assert!(
            shown.iter().any(|t| t.starts_with(row)),
            "the sheet lists {row}: {shown:?}"
        );
    }
    tap(&mut app, Action::Choices(ChoicesPress::Row(vstep::PRESET)));
    assert_eq!(app.sheet, None, "a press on a row closes the sheet");
    assert_eq!(open(&app), Some(vstep::PRESET), "and opens its step");
}

#[test]
fn add_a_key_offers_other_forms_above_the_field_and_opens_a_sheet() {
    let mut app = panel();
    app.press(Action::Entry(None));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Entry);
    let forms = on_panel(&mut app, Action::EntryOtherForms(Opening::AddAKey))
        .expect("Other forms is on the panel without scrolling");
    let scan = on_panel(&mut app, Action::ScanSeed).expect("Scan a SeedQR is on the panel");
    assert!(
        scan.1 < forms.1 && forms.1 < H * 3 / 5,
        "Other forms stands under Scan, above the field"
    );
    tap(&mut app, Action::EntryOtherForms(Opening::AddAKey));
    assert_eq!(app.sheet, Some(Sheet::OtherForms(Opening::AddAKey)));
    let shown: Vec<String> = app
        .drawn_text_boxes()
        .plain()
        .into_iter()
        .map(|(t, _)| t)
        .collect();
    use faraday_core::forms::Form;
    let up_front = [Form::Words, Form::Codex32];
    for form in Form::ALL {
        let listed = shown.iter().any(|t| t == form.name());
        assert_eq!(
            listed,
            !up_front.contains(&form),
            "the sheet lists the forms not shown first, and only them: {} in {shown:?}",
            form.name()
        );
    }
    let at = Form::ALL.iter().position(|f| *f == Form::Slip39).unwrap() as u8;
    tap(&mut app, Action::EntryForm(at));
    assert_eq!(app.sheet, None, "choosing a form closes the sheet");
}

#[test]
fn checks_backup_menu_opens_its_other_forms_as_a_sheet_on_the_panel() {
    use faraday_core::check::{Backup, Press, Row};
    use faraday_core::forms::Form;
    let mut app = panel();
    app.press(Action::Check(Press::Row(Row::Backup)));
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::CheckBackups));
    app.press(Action::Check(Press::Other));
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::OtherForms(Opening::CheckBackup)));
    let shown: Vec<String> = app
        .drawn_text_boxes()
        .plain()
        .into_iter()
        .map(|(t, _)| t)
        .collect();
    for form in [Form::Slip39, Form::Xor, Form::Krux] {
        assert!(shown.iter().any(|t| t == form.name()), "{}", form.name());
    }
    assert!(
        !shown.iter().any(|t| t == Form::Codex32.name()),
        "codex32 is shown first, not under Other forms"
    );
    assert!(app.offers(Action::Check(Press::Form(Backup::Form(Form::Slip39)))));
}
