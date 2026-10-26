//! Randomness of a person's own, mixed into the session's
//! (`docs/OWN-RANDOMNESS.md` §2.3, F86): what is added changes every byte
//! the session draws, never takes the system's bytes out, makes nothing
//! before the system's first answer, and goes at a lock; with nothing
//! added the bytes are what they were before it existed.
//!
//! The known answers were computed with Python's `hashlib` from §2.3's
//! definitions, independently of this code: SHA-256("faraday own" ‖ acc
//! ‖ commitment) from an all-zero acc, SHA-256("faraday reseed" ‖ seed ‖
//! acc), SHA-256("faraday fresh" ‖ what ‖ answer ‖ acc), and a seed draw
//! SHA-256(what ‖ seed ‖ count).

use crate::rounds::{at, device as musig_device, our_nonce};
use crate::vault_seal::{device as vault_device, nonces, open_and_change, sealed};
use faraday_core::fresh::page::Act;
use faraday_core::fresh::{OwnCounts, OwnSource};
use faraday_core::keygen::Way;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_entropy::{CameraNoise, CoinFlips, DiceRolls};
use faraday_shell_api::{App, Command, EntropyBytes, Event, Key};

/// The system's first answer, the session's seed.
const SEED: [u8; 32] = [0x11; 32];
/// The system's second answer, waiting in the pool.
const POOLED: [u8; 32] = [0x22; 32];
/// What the fresh bytes are taken for.
const WHAT: &[u8] = b"faraday test";
/// What the seed's draw is for, and its count.
const DRAW: &[u8] = b"faraday test draw";
const COUNT: &[u8] = b"count";

/// SHA-256("faraday fresh" ‖ WHAT ‖ POOLED).
const FRESH_PLAIN: &str = "bf1ad502ef69b59b91987efa113f86dc832293d47fa42f3a604743799fdf4c07";
/// SHA-256(DRAW ‖ SEED ‖ COUNT).
const DRAW_PLAIN: &str = "5e1e6e312247cd1efb95edae7db2c66f3a6588cde135c90ee91e75c29679364f";
/// SHA-256("123456"): the dice commitment of the rolls 1 to 6.
const DICE_COMMITMENT: &str = "8d969eef6ecad3c29a3a629280e686cf0c3f5d5a86aff3ca12020c923adc6c92";
/// SHA-256("HTTH"): the coin commitment of heads, tails, tails, heads.
const COINS_COMMITMENT: &str = "caf9dbbd728bd060896af6384a77e548fb1d79b96928d2345b5d30a65675a546";
/// With the dice added: the draw from SHA-256("faraday reseed" ‖ SEED ‖
/// acc1), and the fresh bytes with acc1 after the answer.
const DRAW_DICE: &str = "7b795d54616b572836b470cc594e232e72c1a24c0bbae282c5186b76b602d031";
const FRESH_DICE: &str = "7e3f2c6b2ebffb02a1d505b8004ffefbacfb7806df8032b1b93589e3693a6c65";
/// With the dice, then the coins added: acc2 chained from acc1, the seed
/// hashed again with it.
const DRAW_DICE_COINS: &str = "c1b0177f5056c4ec9f56f550f1743f96366034354f5ba066b03ffa105f9d09b6";
const FRESH_DICE_COINS: &str = "881bfdaab65025a43c5b28bcf6b98c228a2b05c2d67c911098962b13a24dfc53";

fn hex(b: [u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn answer(app: &mut Faraday, bytes: [u8; 32]) {
    app.event(Event::Entropy(EntropyBytes::new(bytes)));
}

/// The rolls 1 to 6, added as Add randomness adds a batch.
fn add_dice(app: &mut Faraday) {
    let mut rolls = DiceRolls::new();
    for r in 1..=6 {
        assert!(rolls.push(r));
    }
    assert_eq!(hex(rolls.commitment()), DICE_COMMITMENT);
    assert!(app.own_add(OwnSource::Dice, &rolls.commitment(), 6));
}

/// Heads, tails, tails, heads.
fn add_coins(app: &mut Faraday) {
    let mut flips = CoinFlips::new();
    for heads in [true, false, false, true] {
        assert!(flips.push(heads));
    }
    assert_eq!(hex(flips.commitment()), COINS_COMMITMENT);
    assert!(app.own_add(OwnSource::Coins, &flips.commitment(), 4));
}

/// One camera frame with every luma value in it.
fn add_frame(app: &mut Faraday) {
    let mut camera = CameraNoise::new();
    let luma: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
    assert!(camera.push(&luma).1);
    assert!(app.own_add(OwnSource::Camera, &camera.commitment(), 1));
}

/// The seed's draw (which uses nothing up), then the fresh bytes (which
/// take the pooled answer).
fn drawn(app: &mut Faraday) -> (String, String) {
    let draw = testkit::seed_draw(app, DRAW, COUNT).expect("seeded");
    let fresh = testkit::fresh(app, WHAT).expect("an answer pooled");
    (hex(draw), hex(fresh))
}

/// The code of a new keyboard, number `id`, plugged in now.
fn keyboard_code(app: &mut Faraday, id: u32) -> Option<String> {
    app.storage(StorageEvent::NewInput {
        id,
        name: "USB Keyboard".into(),
        keyboard: true,
        pointer: false,
    });
    app.new_inputs().iter().find(|i| i.id == id)?.code.clone()
}

#[test]
fn with_nothing_added_the_bytes_are_the_systems_as_before() {
    let mut app = testkit::started();
    answer(&mut app, SEED);
    answer(&mut app, POOLED);
    assert_eq!(app.own_counts(), OwnCounts::default());
    assert_eq!(
        drawn(&mut app),
        (DRAW_PLAIN.to_string(), FRESH_PLAIN.to_string())
    );
}

#[test]
fn dice_added_are_mixed_into_the_seed_and_the_fresh_bytes_as_defined() {
    // Added after the system's answers.
    let mut after = testkit::started();
    answer(&mut after, SEED);
    answer(&mut after, POOLED);
    add_dice(&mut after);
    // Added before them: the seed is made from both when the answer
    // comes, to the same bytes.
    let mut before = testkit::started();
    add_dice(&mut before);
    answer(&mut before, SEED);
    answer(&mut before, POOLED);
    let want = (DRAW_DICE.to_string(), FRESH_DICE.to_string());
    assert_eq!(drawn(&mut after), want);
    assert_eq!(drawn(&mut before), want);
}

#[test]
fn a_second_batch_chains_onto_the_first_and_reseeds_again() {
    let mut app = testkit::started();
    answer(&mut app, SEED);
    answer(&mut app, POOLED);
    add_dice(&mut app);
    add_coins(&mut app);
    assert_eq!(
        drawn(&mut app),
        (DRAW_DICE_COINS.to_string(), FRESH_DICE_COINS.to_string())
    );
    assert_eq!(
        app.own_counts(),
        OwnCounts {
            dice_rolls: 6,
            coin_flips: 4,
            ..OwnCounts::default()
        }
    );
}

#[test]
fn a_batch_of_nothing_is_not_taken() {
    let mut app = testkit::started();
    answer(&mut app, SEED);
    answer(&mut app, POOLED);
    assert!(!app.own_add(OwnSource::Dice, &DiceRolls::new().commitment(), 0));
    assert_eq!(app.own_counts(), OwnCounts::default());
    assert_eq!(
        drawn(&mut app),
        (DRAW_PLAIN.to_string(), FRESH_PLAIN.to_string())
    );
}

#[test]
fn dice_added_change_the_seal_nonce_the_keyboard_code_and_a_new_keys_bytes() {
    let session = |dice: bool| {
        let mut app = vault_device(testkit::test_vault().unwrap());
        answer(&mut app, SEED);
        answer(&mut app, POOLED);
        if dice {
            add_dice(&mut app);
        }
        let code = keyboard_code(&mut app, 4).expect("a code once seeded");
        let key = testkit::fresh(&mut app, b"vault").expect("an answer pooled");
        app.press(Action::InputIgnore(4));
        open_and_change(&mut app);
        app.press(Action::Lock);
        let file = sealed(&app).expect("the vault was not sealed");
        (code, key, nonces(&file))
    };
    let (plain, mixed) = (session(false), session(true));
    assert_ne!(plain.0, mixed.0, "the keyboard's code");
    assert_ne!(plain.1, mixed.1, "a new key's fresh bytes");
    let resealed: Vec<usize> = (0..plain.2.len())
        .filter(|&i| plain.2[i] != mixed.2[i])
        .collect();
    assert_eq!(resealed.len(), 1, "the slot sealed again has another nonce");
    // The same session again is the same: only the dice made the change.
    assert_eq!(session(true).0, mixed.0);
}

#[test]
fn the_same_dice_with_other_system_bytes_give_other_bytes() {
    let session = |seed: [u8; 32]| {
        let mut app = testkit::started();
        add_dice(&mut app);
        answer(&mut app, seed);
        answer(&mut app, POOLED);
        let code = keyboard_code(&mut app, 4).expect("a code once seeded");
        (code, drawn(&mut app))
    };
    let (a, b) = (session(SEED), session([0x12; 32]));
    assert_ne!(a.0, b.0, "the keyboard's code");
    assert_ne!(a.1.0, b.1.0, "the seed's draw");
    // The pooled answer, with the same dice, differs from another one.
    let mut c = testkit::started();
    add_dice(&mut c);
    answer(&mut c, SEED);
    answer(&mut c, [0x23; 32]);
    assert_ne!(drawn(&mut c).1, a.1.1, "the fresh bytes");
}

#[test]
fn what_is_added_before_the_system_answers_makes_nothing() {
    let mut app = testkit::started();
    add_dice(&mut app);
    add_frame(&mut app);
    assert!(!app.seeded());
    assert_eq!(testkit::seed_draw(&app, DRAW, COUNT), None);
    assert_eq!(testkit::fresh(&mut app, WHAT), None);
    assert_eq!(keyboard_code(&mut app, 4), None, "no code for a keyboard");
    app.press(Action::InputIgnore(4));
    // After the answer, everything does.
    answer(&mut app, SEED);
    answer(&mut app, POOLED);
    assert!(app.seeded());
    assert!(keyboard_code(&mut app, 5).is_some());
    assert!(testkit::seed_draw(&app, DRAW, COUNT).is_some());
    assert!(testkit::fresh(&mut app, WHAT).is_some());
}

#[test]
fn a_vault_does_not_seal_on_what_was_added_alone() {
    let mut app = vault_device(testkit::test_vault().unwrap());
    add_dice(&mut app);
    add_frame(&mut app);
    open_and_change(&mut app);
    app.press(Action::Lock);
    assert_eq!(sealed(&app), None);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    answer(&mut app, SEED);
    app.press(Action::Lock);
    assert!(sealed(&app).is_some());
}

#[test]
fn a_signing_round_does_not_open_on_what_was_added_alone() {
    let mut app = musig_device(false);
    add_dice(&mut app);
    add_frame(&mut app);
    app.press(Action::StartSpend(at(&app, "musig-unsigned.psbt")));
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().musig.is_none());
    assert!(our_nonce(&app).is_none());
    answer(&mut app, SEED);
    app.press(Action::SignHere);
    assert!(our_nonce(&app).is_some());
}

#[test]
fn new_keys_this_device_makes_no_words_on_what_was_added_alone() {
    let mut app = crate::keygen::opened();
    add_dice(&mut app);
    add_frame(&mut app);
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Device.index()));
    app.press(Action::KNext);
    app.press(Action::KNext);
    assert!(app.keygen().unwrap().mnemonic.is_none());
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Waiting for randomness"),
        "the foot button says what it waits for (F102)"
    );
    answer(&mut app, SEED);
    app.press(Action::KNext);
    assert!(app.keygen().unwrap().mnemonic.is_some());
}

/// New key's This device with the dice added: SHA-256("faraday device
/// own" ‖ SEED ‖ acc1), the bytes the shell answered with the person's
/// accumulator hashed in (§2.3).
const DEVICE_DICE: &str = "71018a2755836a806283b23356523a88c063153b225d6eeffe5176180be792b1";

/// New key on This device at 12 words, answered with SEED: its words.
fn device_words(app: &mut Faraday) -> Vec<u16> {
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Device.index()));
    app.press(Action::KNext);
    app.press(Action::KNext);
    answer(app, SEED);
    app.press(Action::KNext);
    app.keygen()
        .unwrap()
        .mnemonic
        .as_ref()
        .unwrap()
        .indices()
        .to_vec()
}

/// The 12 words This device makes of `bytes`: SHA-256 of them, its first
/// 16 bytes.
fn words_of(bytes: [u8; 32]) -> Vec<u16> {
    use faraday_bip::bip39::{Language, Mnemonic};
    use faraday_bip::bitcoin::hashes::{Hash, sha256};
    let h = sha256::Hash::hash(&bytes).to_byte_array();
    Mnemonic::from_entropy(Language::English, &h[..16])
        .unwrap()
        .indices()
        .to_vec()
}

fn unhex(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
    }
    out
}

#[test]
fn new_keys_this_device_mixes_in_what_was_added_and_says_so() {
    let mut plain = crate::keygen::opened();
    assert_eq!(device_words(&mut plain), words_of(SEED), "nothing added");
    assert!(
        !plain
            .drawn_texts()
            .iter()
            .any(|t| t.contains("with your own")),
        "nothing added, nothing said"
    );

    let mut mixed = crate::keygen::opened();
    add_dice(&mut mixed);
    assert_eq!(
        device_words(&mut mixed),
        words_of(unhex(DEVICE_DICE)),
        "the dice mixed in as §2.3 defines"
    );
    mixed.press(Action::KStep(faraday_core::keygen::kstep::ENTER));
    assert!(
        mixed
            .drawn_texts()
            .iter()
            .any(|t| t.ends_with("· with your own")),
        "This device's line says so: {:?}",
        mixed.drawn_texts()
    );
}

#[test]
fn a_lock_wipes_what_was_added() {
    let mut app = testkit::started();
    answer(&mut app, SEED);
    answer(&mut app, POOLED);
    add_dice(&mut app);
    assert!(app.own_counts().any());
    app.press(Action::Lock);
    assert!(app.restart_requested());
    assert_eq!(app.own_counts(), OwnCounts::default());
    // The pooled answer is no longer mixed with anything.
    assert_eq!(
        testkit::fresh(&mut app, WHAT).map(hex).as_deref(),
        Some(FRESH_PLAIN)
    );
}

// ----- Add randomness and the status line (§2.2, §2.4; test 5) -----

fn own(a: Act) -> Action {
    Action::Own(a)
}

/// The status line on show.
fn status_shown(app: &mut Faraday) -> Option<String> {
    app.drawn_texts()
        .into_iter()
        .find(|t| t.starts_with("System: "))
}

/// A display tall enough for the whole of Settings.
fn tall() -> faraday_shell_api::DisplayInfo {
    faraday_shell_api::DisplayInfo {
        width: 1366,
        height: 2400,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }
}

fn commands(app: &mut Faraday) -> Vec<Command> {
    std::iter::from_fn(|| app.poll_command()).collect()
}

#[test]
fn settings_randomness_row_counts_what_the_page_added_and_none_after_a_lock() {
    let mut app = testkit::started();
    // Tall enough that Settings shows the row without scrolling.
    app.event(Event::Display(tall()));
    app.event(Event::EntropySource("getrandom(2)"));
    answer(&mut app, SEED);
    app.press(Action::Nav(Screen::Settings));
    crate::common::settings_section(&mut app, "Randomness");
    assert_eq!(
        status_shown(&mut app).as_deref(),
        Some("System: getrandom(2) · your own: none")
    );
    app.press(own(Act::Open));
    assert_eq!(app.screen, Screen::Randomness);
    // No camera here: the page opens on Dice.
    for i in 0..99u8 {
        app.press(own(Act::Roll(i % 6 + 1)));
    }
    assert_eq!(app.own_counts(), OwnCounts::default(), "nothing before Add");
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "In this batch: 99 dice rolls"),
        "{:?}",
        app.drawn_texts()
    );
    app.press(own(Act::Add));
    assert_eq!(app.own_counts().dice_rolls, 99);
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "In this batch: 0 dice rolls"),
        "the batch goes once it is added"
    );
    app.press(Action::Back);
    assert_eq!(app.screen, Screen::Settings);
    assert_eq!(
        status_shown(&mut app).as_deref(),
        Some("System: getrandom(2) · your own: 99 dice rolls")
    );
    app.press(Action::Lock);
    assert!(app.restart_requested());
    app.press(Action::Nav(Screen::Settings));
    crate::common::settings_section(&mut app, "Randomness");
    assert_eq!(
        status_shown(&mut app).as_deref(),
        Some("System: getrandom(2) · your own: none")
    );
}

#[test]
fn dice_typed_on_the_page_mix_in_as_the_known_answers_say() {
    let mut app = testkit::started();
    answer(&mut app, SEED);
    answer(&mut app, POOLED);
    app.press(Action::Nav(Screen::Settings));
    crate::common::settings_section(&mut app, "Randomness");
    app.press(own(Act::Open));
    // 7 is no die's face; Backspace takes the second 6 back.
    crate::common::type_text(&mut app, "12345667");
    app.event(Event::Key(Key::Backspace));
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.own_counts().dice_rolls, 6);
    assert_eq!(
        drawn(&mut app),
        (DRAW_DICE.to_string(), FRESH_DICE.to_string()),
        "the rolls 1 to 6, as the cross-checked answers"
    );
}

#[test]
fn coins_and_cards_are_counted_and_a_card_already_out_is_refused() {
    let mut app = testkit::started();
    answer(&mut app, SEED);
    app.press(Action::Nav(Screen::Settings));
    crate::common::settings_section(&mut app, "Randomness");
    app.press(own(Act::Open));
    app.press(own(Act::Source(OwnSource::Coins)));
    for h in [true, false, false, true] {
        app.press(own(Act::Flip(h)));
    }
    app.press(own(Act::Add));
    app.press(own(Act::Source(OwnSource::Cards)));
    app.press(own(Act::Rank(0)));
    app.press(own(Act::Suit(0)));
    app.press(own(Act::Rank(0)));
    app.press(own(Act::Suit(0)));
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "That card is already out of this deck")
    );
    app.press(own(Act::Rank(12)));
    app.press(own(Act::Suit(3)));
    app.press(own(Act::Add));
    let c = app.own_counts();
    assert_eq!((c.coin_flips, c.cards), (4, 2));
    let line = status_shown(&mut app);
    assert!(
        line.as_deref()
            .is_some_and(|s| s.ends_with("your own: 4 coin flips, 2 cards")),
        "{line:?}"
    );
}

#[test]
fn with_a_camera_the_page_opens_on_it_and_each_picture_is_added_as_taken() {
    let mut app = testkit::started();
    answer(&mut app, SEED);
    answer(&mut app, POOLED);
    app.storage(StorageEvent::Cameras(vec![(
        "cam0".to_string(),
        "Camera".to_string(),
    )]));
    app.press(Action::Nav(Screen::Settings));
    crate::common::settings_section(&mut app, "Randomness");
    let _ = commands(&mut app);
    app.press(own(Act::Open));
    assert!(commands(&mut app).contains(&Command::CameraOn));
    // A flat picture is refused, as New key refuses it.
    app.event(Event::CameraFrame {
        width: 64,
        height: 64,
        luma: vec![7; 4096],
        chroma: None,
    });
    app.event(Event::Key(Key::Char(' ')));
    assert_eq!(app.own_counts(), OwnCounts::default());
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Too little variation in that picture")
    );
    let luma: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
    app.event(Event::CameraFrame {
        width: 64,
        height: 64,
        luma,
        chroma: None,
    });
    app.press(own(Act::Frame));
    assert_eq!(app.own_counts().camera_frames, 1);
    // The same bytes as the frame added through the session directly.
    let mut direct = testkit::started();
    answer(&mut direct, SEED);
    answer(&mut direct, POOLED);
    add_frame(&mut direct);
    assert_eq!(drawn(&mut app), drawn(&mut direct));
    app.press(Action::Back);
    assert!(commands(&mut app).contains(&Command::CameraOff));
}

#[test]
fn new_keys_generator_offers_the_sources_on_its_card_and_takes_its_bytes_again_mixed() {
    const ADDED: &str = "Randomness added to the pool";
    let mut app = crate::keygen::opened();
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Device.index()));
    app.press(Action::KNext);
    app.press(Action::KNext);
    answer(&mut app, SEED);
    assert!(
        !crate::common::offered(&mut app, own(Act::Open)),
        "no button opens a page for it"
    );
    for s in [OwnSource::Dice, OwnSource::Coins, OwnSource::Cards] {
        assert!(app.offers(own(Act::Source(s))), "{s:?} on the card");
    }
    assert!(
        !app.offers(own(Act::Roll(1))),
        "no pad until a source is chosen"
    );
    // Keys typed before a source is chosen collect nothing.
    crate::common::type_text(&mut app, "123456");
    assert!(!app.own_counts().any(), "nothing taken passively");
    assert!(!crate::common::has(&mut app, ADDED), "nothing added yet");
    app.press(own(Act::Source(OwnSource::Dice)));
    assert!(
        crate::common::offered(&mut app, own(Act::Roll(1))),
        "the dice pad on the card"
    );
    crate::common::type_text(&mut app, "123456");
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.screen, Screen::KeyGen, "added on the card itself");
    assert!(crate::common::has(&mut app, ADDED), "the card says so");
    // The bytes taken before the dice go; the next answer is mixed.
    answer(&mut app, SEED);
    app.press(Action::KNext);
    let words = app
        .keygen()
        .unwrap()
        .mnemonic
        .as_ref()
        .unwrap()
        .indices()
        .to_vec();
    assert_eq!(words, words_of(unhex(DEVICE_DICE)));
}

#[test]
fn new_keys_generator_with_a_camera_starts_it_only_when_camera_is_chosen() {
    let mut app = crate::keygen::opened();
    app.storage(StorageEvent::Cameras(vec![(
        "cam0".to_string(),
        "Camera".to_string(),
    )]));
    app.press(Action::KWords(12));
    let _ = commands(&mut app);
    app.press(Action::KWay(Way::Device.index()));
    assert!(
        !commands(&mut app).contains(&Command::CameraOn),
        "no source chosen: no camera unasked"
    );
    let camera = own(Act::Source(OwnSource::Camera));
    assert!(crate::common::offered(&mut app, camera));
    assert!(
        !commands(&mut app).contains(&Command::CameraOn),
        "nor on drawing"
    );
    app.press(camera);
    assert!(commands(&mut app).contains(&Command::CameraOn));
    app.press(Action::KStep(faraday_core::keygen::kstep::SOURCE));
    assert!(
        commands(&mut app).contains(&Command::CameraOff),
        "leaving the card stops it"
    );
}

#[test]
fn new_keys_generator_skip_moves_on_with_nothing_added() {
    let mut app = crate::keygen::opened();
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Device.index()));
    answer(&mut app, SEED);
    let skip = own(Act::Skip);
    assert!(crate::common::offered(&mut app, skip), "Skip on the card");
    app.press(skip);
    let k = app.keygen().unwrap();
    assert_eq!(k.open, Some(faraday_core::keygen::kstep::KEY), "on to Key");
    assert!(!k.with_own && !app.own_counts().any(), "nothing added");
    let words = k.mnemonic.as_ref().unwrap().indices().to_vec();
    assert_eq!(words, words_of(SEED), "the system's bytes alone");
}
