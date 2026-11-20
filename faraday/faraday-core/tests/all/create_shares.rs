//! Make a new wallet's Backup card and what it fixes (F240,
//! `docs/CODEX32.md` §6.9): a single key asks how its key is backed up,
//! words or codex32 shares; New key then makes it that way, and no key a
//! person puts in its slot is refused at the end. Kind offers Single key
//! and Multisig, no shares kind, and Restore offers none.

use crate::common::{Shell, add_key, keep_time, process};
use faraday_core::codex32::{C32, Card, SPLITS, Split, WAYS, Way};
use faraday_core::create::{BackupForm, NewKind};
use faraday_core::keygen::{Start, kstep};
use faraday_core::{Action, Faraday, Screen, cstep};
use faraday_shell_api::{
    App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware, TouchPhase,
};
use faraday_ui::keyboard::OskPress;

fn app() -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
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
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x50 + i; 32])));
    }
    app
}

fn kind_index(kind: NewKind) -> u8 {
    NewKind::ALL.iter().position(|k| *k == kind).unwrap() as u8
}

/// Make a new wallet opened, `kind` chosen: a single key's Backup card is
/// open.
fn on_backup_of(mut app: Faraday, kind: NewKind) -> Faraday {
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::CreateWallet);
    app.press(Action::CKind(kind_index(kind)));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::BACKUP_FORM));
    app
}

fn on_backup() -> Faraday {
    on_backup_of(app(), NewKind::NativeSegwit)
}

/// The Backup card answered with `form`, No to vaults: Keys.
fn on_keys_of(mut app: Faraday, kind: NewKind, form: BackupForm) -> Faraday {
    app = on_backup_of(app, kind);
    app.press(Action::CBackupForm(form.index()));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KEYS));
    app
}

/// A multisig's Keys card, its first cosigner slot's New key opened.
fn on_multisig_slot() -> Faraday {
    let mut app = app();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::CreateWallet);
    app.press(Action::CKind(kind_index(NewKind::Multi)));
    app.press(Action::CNext(cstep::QUORUM));
    open_new_key(&mut app);
    app
}

/// Frames drawn, a tick before each, until a step's fade has run out: the
/// step that changed shows as the card.
fn through_fade(app: &mut Faraday) {
    let mut t = app.now_ms();
    for _ in 0..40 {
        t += 16;
        app.event(Event::Tick { now_ms: t });
        let _ = app.frame();
    }
}

/// New key opened for the first slot, whichever way Keys got there.
fn open_new_key(app: &mut Faraday) {
    if app.screen != Screen::KeyGen {
        app.press(Action::KeyGen(Some(0)));
    }
    assert_eq!(app.screen, Screen::KeyGen);
    let _ = app.frame();
}

/// Make codex32 shares opened for a single key's slot, on Scheme.
pub(crate) fn on_scheme_of(app: Faraday, kind: NewKind) -> Faraday {
    let mut app = on_keys_of(app, kind, BackupForm::Codex32);
    open_new_key(&mut app);
    app.press(Action::KStart(
        Start::Make(faraday_core::forms::Form::Codex32).index(),
    ));
    let k = app.keygen().expect("New key");
    assert_eq!(k.open, Some(kstep::CODEX32));
    assert_eq!(k.codex32.as_ref().unwrap().open(), Some(Card::Scheme));
    app
}

/// [`on_scheme_of`] a native SegWit key, 2 of 3 chosen, on the identifier.
pub(crate) fn on_identifier_of(app: Faraday) -> Faraday {
    let mut app = on_scheme_of(app, NewKind::NativeSegwit);
    key_scheme(&mut app);
    app
}

fn key_scheme(app: &mut Faraday) {
    let i = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
    app.press(Action::Codex32(C32::Split(i as u8)));
    app.press(Action::Codex32(C32::Next));
}

pub(crate) fn key(app: &mut Faraday, ch: char) {
    app.press(Action::Codex32(C32::Key(OskPress::Char(ch))));
}

fn starts(app: &Faraday) -> Vec<Start> {
    app.keygen().expect("New key").starts()
}

const WORDS: &str = "Words (BIP-39)";
const NAME: &str = "Shares (codex32)";
const TIP: &str = "codex32 is the only method that makes a seed and its shares fully by hand, the checksum included.";

#[test]
fn kind_shows_single_key_and_multisig_with_none_ticked_and_no_shares_row() {
    let mut app = app();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::CreateWallet);
    let c = app.create().expect("Make a new wallet");
    assert_eq!(c.open, Some(cstep::KIND), "Kind is open");
    assert!(c.kind_unchosen, "no kind is ticked");
    let texts = app.drawn_texts();
    let at = |name: &str| texts.iter().position(|t| t == name);
    let (single, multi) = (
        at("Single key").expect("Single key"),
        at("Multisig").expect("Multisig"),
    );
    assert!(single < multi, "{texts:?}");
    assert!(
        !texts.iter().any(|t| t.contains("backup in shares")),
        "no shares kind: {texts:?}"
    );
    assert_eq!(
        NewKind::create_rows(false),
        vec![0, 4],
        "Single key and Multisig before More kinds"
    );
}

#[test]
fn a_single_key_opens_the_backup_card_with_words_and_shares_and_neither_ticked() {
    for kind in [NewKind::NativeSegwit, NewKind::Taproot] {
        let mut app = on_backup_of(app(), kind);
        let c = app.create().unwrap();
        assert_eq!(c.backup, None, "neither is ticked");
        let _ = app.frame();
        let texts = app.drawn_texts();
        assert!(texts.iter().any(|t| t == WORDS), "{texts:?}");
        assert!(texts.iter().any(|t| t == NAME), "Shares shown: {texts:?}");
        assert!(!texts.iter().any(|t| t == "More ways"), "{texts:?}");
        // No way on until one is pressed, and only the cards up to this
        // one show.
        app.press(Action::CNext(cstep::BACKUP_FORM));
        let c = app.create().unwrap();
        assert_eq!(c.backup, None);
        assert_eq!(c.open, Some(cstep::BACKUP_FORM), "Next waits");
        assert_eq!(
            app.create_cards().last(),
            Some(&cstep::BACKUP_FORM),
            "no card after Backup shows"
        );
    }
}

#[test]
fn pressing_words_or_shares_on_the_backup_card_goes_on_with_that_type() {
    for form in [BackupForm::Words, BackupForm::Codex32] {
        let mut app = on_backup_of(app(), NewKind::NativeSegwit);
        app.press(Action::CBackupForm(form.index()));
        let c = app.create().unwrap();
        assert_eq!(c.backup, Some(form));
        assert_eq!(c.open, Some(cstep::KEYS), "the next card opens");
    }
}

#[test]
fn a_multisig_has_a_quorum_and_no_backup_card() {
    let mut app = app();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(kind_index(NewKind::Multi)));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::QUORUM));
    app.press(Action::CBackupForm(BackupForm::Codex32.index()));
    assert_eq!(
        app.create().unwrap().backup,
        None,
        "a multisig has no backup card; the press changes nothing"
    );
}

#[test]
fn words_make_new_key_offer_bip39_words_and_no_codex32() {
    let mut app = on_keys_of(app(), NewKind::NativeSegwit, BackupForm::Words);
    open_new_key(&mut app);
    assert_eq!(
        starts(&app),
        vec![
            Start::Make(faraday_core::forms::Form::Words),
            Start::Type(faraday_core::forms::Form::Words),
            Start::Scan,
            Start::Type(faraday_core::forms::Form::Krux),
            Start::Type(faraday_core::forms::Form::Codex32),
            Start::Type(faraday_core::forms::Form::Slip39),
            Start::Type(faraday_core::forms::Form::Xor)
        ],
        "the fixed make, and the ways to add a key you have"
    );
    assert_eq!(
        app.keygen().unwrap().open,
        Some(kstep::LENGTH),
        "Start is answered by the Backup card: the flow opens on the count"
    );
    assert_eq!(
        app.keygen().unwrap().start,
        Start::Make(faraday_core::forms::Form::Words)
    );
    app.press(Action::KStep(kstep::START));
    through_fade(&mut app);
    assert!(app.offers(Action::KStart(
        Start::Make(faraday_core::forms::Form::Words).index()
    )));
    assert!(!app.offers(Action::KStart(
        Start::Make(faraday_core::forms::Form::Codex32).index()
    )));
}

#[test]
fn a_wallet_made_from_words_has_no_backup_row_on_check() {
    let mut app = app();
    add_key(&mut app, "bacon");
    let mut app = on_keys_of(app, NewKind::NativeSegwit, BackupForm::Words);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::CStep(cstep::KEYS));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    let i = app.create().unwrap().built.expect("the wallet is made");
    assert!(app.create_shares_set(i).is_none());
    let _ = app.frame();
    assert!(
        !app.drawn_texts().iter().any(|t| t.starts_with("codex32 ")),
        "no codex32 Backup row"
    );
}

#[test]
fn codex32_make_new_key_offer_codex32_and_no_bip39_words_to_make() {
    let mut app = on_keys_of(app(), NewKind::NativeSegwit, BackupForm::Codex32);
    open_new_key(&mut app);
    assert_eq!(
        starts(&app),
        vec![
            Start::Make(faraday_core::forms::Form::Codex32),
            Start::Type(faraday_core::forms::Form::Words),
            Start::Scan,
            Start::Type(faraday_core::forms::Form::Krux),
            Start::Type(faraday_core::forms::Form::Codex32),
            Start::Type(faraday_core::forms::Form::Slip39),
            Start::Type(faraday_core::forms::Form::Xor)
        ],
        "the fixed make, and the ways to add a key you have"
    );
    assert_eq!(
        app.keygen().unwrap().start,
        Start::Make(faraday_core::forms::Form::Codex32)
    );
    app.press(Action::KStep(kstep::START));
    through_fade(&mut app);
    assert!(app.offers(Action::KStart(
        Start::Make(faraday_core::forms::Form::Codex32).index()
    )));
    assert!(!app.offers(Action::KStart(
        Start::Make(faraday_core::forms::Form::Words).index()
    )));
}

#[test]
fn make_codex32_shares_for_a_wallet_has_no_none_the_secret_alone() {
    let app = on_scheme_of(app(), NewKind::NativeSegwit);
    let splits = app.keygen().unwrap().codex32.as_ref().unwrap().splits();
    let kinds: Vec<Split> = splits.iter().map(|(_, s)| *s).collect();
    assert_eq!(
        kinds,
        vec![Split::TwoOfThree, Split::ThreeOfFive, Split::Other]
    );
    // A press for it is ignored.
    let mut app = app;
    let alone = SPLITS.iter().position(|s| *s == Split::Alone).unwrap();
    app.press(Action::Codex32(C32::Split(alone as u8)));
    assert_ne!(
        app.keygen().unwrap().codex32.as_ref().unwrap().split(),
        Split::Alone,
        "the secret alone is not chosen"
    );
    // On its own it is still offered.
    let mut own = self::app();
    own.press(Action::Nav(Screen::WalletsStart));
    own.press(Action::KeyGen(None));
    own.press(Action::KStart(
        Start::Make(faraday_core::forms::Form::Codex32).index(),
    ));
    let splits = own.keygen().unwrap().codex32.as_ref().unwrap().splits();
    assert_eq!(splits.len(), SPLITS.len());
}

/// BIP 93's vector 2 typed in at 2 of 3 makes a single-key wallet of
/// `kind`, whose Check names the set.
fn the_set_made_makes_the_wallet(kind: NewKind, script: faraday_bip::keys::ScriptType) {
    let all = [
        "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
        "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
        "ms12namedll4f8jlh4e5vdvuldlfxu2jhdnlsm97xvenrxeg",
    ];
    let mut app = on_scheme_of(app(), kind);
    key_scheme(&mut app);
    for ch in "name".chars() {
        key(&mut app, ch);
    }
    app.press(Action::Codex32(C32::Next));
    let w = WAYS.iter().position(|w| *w == Way::Typed).unwrap();
    app.press(Action::Codex32(C32::Way(w as u8)));
    app.press(Action::Codex32(C32::Next));
    for s in &all[..2] {
        for ch in s[9..35].chars() {
            key(&mut app, ch);
        }
    }
    app.press(Action::Codex32(C32::Next));
    let _ = app.frame();
    app.press(Action::Codex32(C32::Next));
    for s in &all {
        for ch in s[3..].chars() {
            key(&mut app, ch);
        }
        let _ = app.frame();
        app.press(Action::Codex32(C32::Next));
    }
    let _ = app.frame();
    app.press(Action::Codex32(C32::Lock));
    faraday_core::testkit::keep_published(&mut app);
    let c = app.create().expect("create");
    let i = c.built.expect("Lock in made the wallet");
    let fp = app.session.keys[0].fingerprint.0;
    let set = app.create_shares_set(i).expect("the set, for Back up");
    assert_eq!(
        (
            set.fingerprint,
            set.identifier.as_str(),
            set.k,
            set.n,
            set.length
        ),
        (fp, "name", 2, 3, 48)
    );
    assert!(
        matches!(
            app.session.wallets[i].policy().template(),
            faraday_bip::policy::Template::Single { script: s } if s == script
        ),
        "a single key of the kind chosen"
    );
    app.press(Action::Codex32(C32::Next));
    assert_eq!(app.screen, Screen::Create);
    // Lock in opened Check.
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::CHECK));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "codex32 name · 2 of 3"),
        "Check's Backup row: {texts:?}"
    );
}

#[test]
fn the_set_made_fills_the_slot_and_check_names_it_native_segwit() {
    the_set_made_makes_the_wallet(
        NewKind::NativeSegwit,
        faraday_bip::keys::ScriptType::NativeSegwit,
    );
}

#[test]
fn the_set_made_fills_the_slot_and_check_names_it_taproot() {
    the_set_made_makes_the_wallet(NewKind::Taproot, faraday_bip::keys::ScriptType::Taproot);
}

/// A key held from codex32 strings, vector 2's A and C.
fn hold_codex32_key(app: &mut Faraday) -> [u8; 4] {
    crate::codex32_kept::key_from_strings(app);
    app.session.keys[0].fingerprint.0
}

#[test]
fn a_codex32_set_you_hold_fills_a_codex32_backups_slot_and_the_wallet_is_made() {
    let mut app = app();
    let fp = hold_codex32_key(&mut app);
    let mut app = on_keys_of(app, NewKind::NativeSegwit, BackupForm::Codex32);
    app.press(Action::CStep(cstep::KEYS));
    // Type codex32 strings is one of the ways offered at Start.
    open_new_key(&mut app);
    app.press(Action::KStep(kstep::START));
    through_fade(&mut app);
    assert!(app.offers(Action::KStart(
        Start::Type(faraday_core::forms::Form::Codex32).index()
    )));
    app.press(Action::Back);
    app.press(Action::CStep(cstep::KEYS));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    let c = app.create().unwrap();
    assert!(c.error.is_none(), "no refusal: {:?}", c.error);
    let i = c.built.expect("the wallet is made");
    assert!(
        app.create_shares_set(i).is_none(),
        "the set was not made here"
    );
}

#[test]
fn a_key_of_words_fills_a_codex32_backups_slot_too() {
    let mut app = app();
    add_key(&mut app, "bacon");
    let mut app = on_keys_of(app, NewKind::Taproot, BackupForm::Codex32);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::CStep(cstep::KEYS));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    let c = app.create().unwrap();
    assert!(c.error.is_none(), "no refusal: {:?}", c.error);
    assert!(c.built.is_some(), "the wallet is made");
}

/// What the hosted Start offers to make, under Other forms too.
fn makes_offered(app: &mut Faraday) -> Vec<Start> {
    app.press(Action::KStep(kstep::START));
    through_fade(app);
    app.press(Action::KOthers);
    let _ = app.frame();
    [
        Start::Make(faraday_core::forms::Form::Words),
        Start::Make(faraday_core::forms::Form::Codex32),
        Start::Make(faraday_core::forms::Form::Slip39),
    ]
    .into_iter()
    .filter(|s| app.offers(Action::KStart(s.index())))
    .collect()
}

#[test]
fn words_chosen_the_hosted_start_makes_words_only_and_still_adds_a_key_you_hold() {
    let mut app = on_keys_of(app(), NewKind::NativeSegwit, BackupForm::Words);
    open_new_key(&mut app);
    assert_eq!(
        makes_offered(&mut app),
        vec![Start::Make(faraday_core::forms::Form::Words)]
    );
    for s in [
        Start::Type(faraday_core::forms::Form::Words),
        Start::Scan,
        Start::Type(faraday_core::forms::Form::Codex32),
        Start::Type(faraday_core::forms::Form::Slip39),
        Start::Type(faraday_core::forms::Form::Xor),
    ] {
        assert!(app.offers(Action::KStart(s.index())), "{s:?} is offered");
    }
}

#[test]
fn codex32_chosen_the_hosted_start_makes_codex32_only_and_still_adds_a_key_you_hold() {
    let mut app = on_keys_of(app(), NewKind::NativeSegwit, BackupForm::Codex32);
    open_new_key(&mut app);
    assert_eq!(
        makes_offered(&mut app),
        vec![Start::Make(faraday_core::forms::Form::Codex32)]
    );
    for s in [
        Start::Type(faraday_core::forms::Form::Words),
        Start::Scan,
        Start::Type(faraday_core::forms::Form::Codex32),
        Start::Type(faraday_core::forms::Form::Slip39),
        Start::Type(faraday_core::forms::Form::Xor),
    ] {
        assert!(app.offers(Action::KStart(s.index())), "{s:?} is offered");
    }
}

/// A share typed for a slot, then Make the rest: the key that comes of
/// it fills that slot and Make a new wallet goes on.
#[test]
fn make_the_rest_on_a_slots_typed_codex32_keeps_the_slot() {
    let mut app = on_keys_of(app(), NewKind::NativeSegwit, BackupForm::Codex32);
    open_new_key(&mut app);
    app.press(Action::KStep(kstep::START));
    through_fade(&mut app);
    app.press(Action::KStart(
        Start::Type(faraday_core::forms::Form::Codex32).index(),
    ));
    assert_eq!(
        app.keygen().unwrap().open,
        Some(kstep::TYPED),
        "the codex32 form"
    );
    let add = |app: &mut Faraday, t: &str| {
        for c in t.chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        app.event(Event::Key(Key::Enter));
    };
    add(&mut app, "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm");
    app.press(Action::Codex32Form(faraday_core::forms::C32Press::Rest));
    add(&mut app, "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn");
    assert_eq!(app.screen, Screen::KeyGen);
    assert_eq!(app.keygen().unwrap().slot, Some(0), "still the slot's");
    let c32 = |app: &mut Faraday, p: C32| app.press(Action::Codex32(p));
    let open = |app: &Faraday| {
        app.keygen()
            .and_then(|k| k.codex32.as_ref())
            .and_then(|m| m.open())
    };
    // On to the sheets and past the strings, whichever card it opens on.
    let mut seen_open = vec![open(&app)];
    for _ in 0..10 {
        match open(&app) {
            Some(Card::Made) => break,
            Some(Card::Strings) => {
                c32(&mut app, C32::Skip);
                c32(&mut app, C32::Skip);
            }
            _ => {
                let _ = app.frame();
                c32(&mut app, C32::Next);
            }
        }
        seen_open.push(open(&app));
    }
    assert_eq!(open(&app), Some(Card::Made), "{seen_open:?}");
    let _ = app.frame();
    let seen = app
        .keygen()
        .and_then(|k| k.codex32.as_ref())
        .map(|m| (m.open(), m.note().map(str::to_owned)));
    c32(&mut app, C32::Lock);
    faraday_core::testkit::keep_published(&mut app);
    assert_eq!(app.session.keys.len(), 1, "the key is loaded: {seen:?}");
    let fp = app.session.keys[0].fingerprint.0;
    let c = app.create().expect("Make a new wallet goes on");
    assert!(c.error.is_none(), "no refusal: {:?}", c.error);
    assert!(
        c.slots[0] == faraday_core::create::Source::Here(fp) || c.built.is_some(),
        "the key fills the slot it was typed for"
    );
    assert_eq!(app.screen, Screen::Create, "Make a new wallet goes on");
}

#[test]
fn a_multisig_cosigner_slot_offers_bip39_words_and_no_slip39_make_and_no_codex32() {
    let app = on_multisig_slot();
    assert_eq!(
        starts(&app),
        vec![
            Start::Make(faraday_core::forms::Form::Words),
            Start::Type(faraday_core::forms::Form::Words),
            Start::Scan,
            Start::Type(faraday_core::forms::Form::Krux),
            Start::Type(faraday_core::forms::Form::Codex32),
            Start::Type(faraday_core::forms::Form::Slip39),
            Start::Type(faraday_core::forms::Form::Xor)
        ]
    );
}

#[test]
fn change_on_start_shows_the_same_choices() {
    let mut app = on_keys_of(app(), NewKind::NativeSegwit, BackupForm::Codex32);
    open_new_key(&mut app);
    let before = starts(&app);
    app.press(Action::KStart(
        Start::Make(faraday_core::forms::Form::Codex32).index(),
    ));
    app.press(Action::Back);
    app.press(Action::KStep(kstep::START));
    through_fade(&mut app);
    assert_eq!(starts(&app), before, "reopened, the same choices");
    assert!(!app.offers(Action::KStart(
        Start::Make(faraday_core::forms::Form::Words).index()
    )));
}

#[test]
fn restore_does_not_offer_a_shares_kind() {
    let mut app = app();
    app.press(Action::RestoreWallet);
    let _ = app.frame();
    assert!(!app.offers(Action::RKind(NewKind::ALL.len() as u8)));
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t.contains("backup in shares")),
        "{texts:?}"
    );
}

// ---- The tooltip on the Backup card's Shares row (F112) ----

/// Make a new wallet open on Backup, with where "codex32" is drawn in the
/// Shares row's name, in pixels, and the clock a second on.
fn on_backup_card() -> (Faraday, (u16, u16)) {
    let mut app = on_backup();
    let now = app.now_ms() + 1000;
    app.event(Event::Tick { now_ms: now });
    let _ = app.frame();
    let (_, (x, y, w, h)) = app
        .drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == NAME)
        .expect("the row's name");
    // "codex32" is inside the brackets: its middle is 11.5 of the name's
    // 16 characters in.
    let word = ((x + w * 72 / 100) as u16, (y + h / 2) as u16);
    (app, word)
}

/// Whether the tooltip's text is drawn, its lines read in order.
fn tip_shows(app: &mut Faraday) -> bool {
    app.drawn_texts().join(" ").contains(TIP)
}

fn tick(app: &mut Faraday, ms: u64) {
    let now = app.now_ms() + ms;
    app.event(Event::Tick { now_ms: now });
}

fn open_card(app: &Faraday) -> Option<u8> {
    app.create().and_then(|c| c.open)
}

#[test]
fn the_pointer_resting_on_codex32_shows_its_tooltip_and_leaving_hides_it() {
    let (mut app, (x, y)) = on_backup_card();
    app.event(Event::Hover { x, y });
    tick(&mut app, 300);
    assert!(!tip_shows(&mut app), "not before the delay");
    tick(&mut app, 300);
    assert!(tip_shows(&mut app), "after it");
    app.event(Event::Hover { x, y: y + 200 });
    assert!(!tip_shows(&mut app), "the pointer gone, the tooltip goes");
    app.event(Event::Hover { x, y });
    tick(&mut app, 600);
    assert!(tip_shows(&mut app));
    app.event(Event::HoverEnd);
    assert!(!tip_shows(&mut app));
    assert_eq!(
        (app.create().unwrap().backup, open_card(&app)),
        (None, Some(cstep::BACKUP_FORM)),
        "hovering pressed nothing"
    );
}

#[test]
fn the_pointer_resting_on_the_info_button_shows_the_tooltip() {
    let (mut app, _) = on_backup_card();
    let (x, y) = app
        .where_offered(Action::CBackupAbout)
        .expect("the info button");
    app.event(Event::Hover { x, y });
    tick(&mut app, 300);
    assert!(!tip_shows(&mut app), "not before the delay");
    tick(&mut app, 300);
    assert!(tip_shows(&mut app), "after it");
    app.event(Event::HoverEnd);
    assert!(!tip_shows(&mut app));
}

/// A finger or a click on (x, y), down and up.
fn tap_at(app: &mut Faraday, (x, y): (u16, u16)) {
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
}

#[test]
fn a_tap_on_the_info_button_shows_the_tooltip_at_once_and_a_press_elsewhere_hides_it() {
    let (mut app, _) = on_backup_card();
    let at = app
        .where_offered(Action::CBackupAbout)
        .expect("the info button");
    tap_at(&mut app, at);
    assert!(tip_shows(&mut app), "at once");
    assert_eq!(
        (app.create().unwrap().backup, open_card(&app)),
        (None, Some(cstep::BACKUP_FORM)),
        "the row is not chosen"
    );
    tick(&mut app, 5000);
    assert!(tip_shows(&mut app), "it stays to be read");
    app.event(Event::Key(Key::Escape));
    assert!(!tip_shows(&mut app), "Escape takes it off");
    tap_at(&mut app, at);
    assert!(tip_shows(&mut app));
    app.event(Event::Touch {
        x: 5,
        y: 5,
        phase: TouchPhase::Down,
    });
    assert!(!tip_shows(&mut app), "a press elsewhere takes it off");
}

#[test]
fn a_tap_on_the_row_beside_the_info_button_chooses_the_row() {
    let (mut app, _) = on_backup_card();
    let row = app
        .where_offered(Action::CBackupForm(BackupForm::Codex32.index()))
        .expect("the row");
    tap_at(&mut app, row);
    tick(&mut app, 100);
    assert_eq!(app.create().unwrap().backup, Some(BackupForm::Codex32));
    assert_eq!(open_card(&app), Some(cstep::KEYS));
    assert!(!tip_shows(&mut app));
}

#[test]
fn the_keyboard_on_the_info_button_shows_its_tooltip_and_escape_hides_it() {
    let (mut app, _) = on_backup_card();
    let mut shown = false;
    for _ in 0..200 {
        app.event(Event::Key(Key::Tab));
        tick(&mut app, 600);
        if tip_shows(&mut app) {
            shown = true;
            break;
        }
    }
    assert!(shown, "Tab reaches the info button, and the tooltip shows");
    assert_eq!(open_card(&app), Some(cstep::BACKUP_FORM), "nothing pressed");
    app.event(Event::Key(Key::Escape));
    assert!(!tip_shows(&mut app), "Escape takes it off");
    assert_eq!(app.screen, Screen::Create, "and does nothing else");
    // Enter presses the button the keyboard is on: the tooltip at once,
    // the row not chosen.
    app.event(Event::Key(Key::Enter));
    assert!(tip_shows(&mut app));
    assert_eq!(open_card(&app), Some(cstep::BACKUP_FORM));
    assert_eq!(app.create().unwrap().backup, None);
    app.event(Event::Key(Key::Escape));
    app.event(Event::Key(Key::Tab));
    tick(&mut app, 600);
    assert!(!tip_shows(&mut app), "the keyboard moved on");
}

#[test]
fn a_long_press_on_codex32_shows_its_tooltip_and_chooses_nothing() {
    let (mut app, (x, y)) = on_backup_card();
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    tick(&mut app, 600);
    assert!(tip_shows(&mut app), "held, the tooltip shows");
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    assert_eq!(
        (app.create().unwrap().backup, open_card(&app)),
        (None, Some(cstep::BACKUP_FORM)),
        "the lift presses nothing"
    );
    tick(&mut app, 3000);
    assert!(tip_shows(&mut app), "it stays to be read");
    app.event(Event::Touch {
        x: 5,
        y: 5,
        phase: TouchPhase::Down,
    });
    assert!(!tip_shows(&mut app), "the next press takes it off");
}

#[test]
fn a_tap_on_codex32_chooses_the_row() {
    let (mut app, (x, y)) = on_backup_card();
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    tick(&mut app, 100);
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    tick(&mut app, 600);
    assert_eq!(app.create().unwrap().backup, Some(BackupForm::Codex32));
    assert_eq!(open_card(&app), Some(cstep::KEYS));
    assert!(!tip_shows(&mut app));
}

// ---- A kept flow across a lock ----

/// Make a new wallet's single key with `form` at Use vaults Yes, one
/// vault, on Bring in the sticks; then locked and the process started
/// again. The shell holding what was kept, and the process.
fn locked_with(form: BackupForm) -> (Shell, Faraday) {
    let mut shell = Shell::new();
    crate::backup_flow_wallet::sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
    app.press(Action::CBackupForm(form.index()));
    app.press(Action::CExistingVault(true));
    assert_eq!(app.screen, Screen::BackupFlow);
    shell.flow(
        &mut app,
        faraday_core::backup_flow::steps::FlowAction::LockBring,
    );
    let app = crate::backup_flow_wallet::restart(&mut shell, &mut app);
    (shell, app)
}

/// The process a start with `kept` makes, as the shell's files stand.
fn restored(shell: &Shell, kept: Vec<(String, Vec<u8>)>) -> Faraday {
    process(shell.boxes.0.clone(), shell.boxes.1.clone(), kept)
}

fn flow_text(kept: &[(String, Vec<u8>)]) -> String {
    let (_, bytes) = kept
        .iter()
        .find(|(n, _)| n == "backup-flow")
        .expect("a flow was kept");
    String::from_utf8(bytes.clone()).unwrap()
}

#[test]
fn a_kept_flow_keeps_the_backup_answer_across_a_lock() {
    for form in BackupForm::ALL {
        let (_, app) = locked_with(form);
        let c = app.create().expect("Make a new wallet is back");
        assert_eq!(c.backup, Some(form), "the answer is kept");
        assert!(c.done[cstep::BACKUP_FORM as usize], "its card is done");
    }
}

/// The kept files with the flow's `wagain` line replaced by `line`.
fn with_again_line(shell: &Shell, line: &str) -> Vec<(String, Vec<u8>)> {
    let mut kept = shell.boxes.2.clone();
    let text = flow_text(&kept);
    let old = text
        .lines()
        .find(|l| l.starts_with("wagain\t"))
        .expect("the kind is kept")
        .to_string();
    kept.iter_mut().find(|(n, _)| n == "backup-flow").unwrap().1 =
        text.replace(&old, line).into_bytes();
    kept
}

#[test]
fn a_flow_kept_with_the_removed_shares_kind_reads_as_single_key_with_a_codex32_backup() {
    let (shell, _) = locked_with(BackupForm::Words);
    // The line as the removed kind wrote it: five fields, kind code 10.
    let app = restored(&shell, with_again_line(&shell, "wagain\t0\t10\t2\t3"));
    let c = app.create().expect("Make a new wallet is back");
    assert_eq!(c.kind, NewKind::NativeSegwit);
    assert_eq!(c.backup, Some(BackupForm::Codex32));
    assert!(c.done[cstep::BACKUP_FORM as usize]);
}

#[test]
fn a_flow_kept_before_the_backup_card_asks_it_again() {
    let (shell, _) = locked_with(BackupForm::Words);
    let app = restored(&shell, with_again_line(&shell, "wagain\t0\t0\t1\t1"));
    let c = app.create().expect("Make a new wallet is back");
    assert_eq!(c.backup, None);
    assert!(!c.done[cstep::BACKUP_FORM as usize]);
}
