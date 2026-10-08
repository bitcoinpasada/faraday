//! The app bar's info button: the one bridge from a working screen to
//! Learn (`docs/PLANNING.md` §16.105).
//!
//! A screen that has a Learn page carries the button in the app bar's
//! trailing slot; a screen with a secret carries the eye there instead;
//! every other screen leaves the slot empty. The button opens the page
//! the screen is about, at the section it is about where the page has
//! one, and the chevron comes back to the screen it was tapped on with
//! the flow it was in untouched.

mod common;

use common::{ABANDON, Harness, PANEL, TILE_LEARN, TILE_SCAN, TILE_TOOLS};
use opensigner_core::load::Step as LoadStep;
use opensigner_core::sign::{Stage, Step as SignStep};
use opensigner_core::strings::EN;
use opensigner_core::{ScreenKind, create, ids};
use osk_shell_api::{Event, FileKind};

/// The 24 words of one member of the committed regtest group, which is
/// the second key a recovery wallet's two paths need.
const SHARE_0_WORDS: &str =
    include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest-share-0.txt");

/// The 12 words every test here loads, as a key.
fn load_key(h: &mut Harness) {
    h.start_load(&ABANDON);
    h.finish_load(None);
}

/// A second key beside it.
fn load_second_key(h: &mut Harness) {
    let words: Vec<&str> = SHARE_0_WORDS.split_whitespace().collect();
    h.start_load_24(&words);
    h.finish_load(None);
    h.go_home();
}

/// Taps the info button on the screen now showing and asserts that it
/// opened `page`: the page's title is on screen, and a page opened this
/// way carries no "Try it" row.
fn open_info(h: &mut Harness, page: &'static osk_learn::Page) {
    assert!(
        h.app.rect_of(ids::INFO).is_some(),
        "{:?} carries no info button",
        h.app.screen()
    );
    h.tap(ids::INFO);
    assert_eq!(h.app.screen(), ScreenKind::LearnPage);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == page.title),
        "the button opened {texts:?} and not {:?}",
        page.title
    );
    assert!(
        h.app.rect_of(ids::LEARN_TRY).is_none(),
        "{:?} opened from a working screen offers its flow again",
        page.title
    );
}

/// The whole round trip on the screen now showing: the button opens
/// `page` and the chevron puts the same screen back.
fn info_and_back(h: &mut Harness, page: &'static osk_learn::Page) {
    let screen = h.app.screen();
    open_info(h, page);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), screen, "the chevron did not come back");
}

/// The lists and pages a person navigates by each open the page for what
/// they hold.
#[test]
fn every_list_opens_the_page_for_what_it_holds() {
    let mut h = Harness::new(PANEL);
    load_key(&mut h);

    h.open_keys();
    info_and_back(&mut h, &EN.learn.words);

    h.open_add(ids::KEYS_ADD);
    info_and_back(&mut h, &EN.learn.words);

    h.go_home();
    h.open_key(0);
    info_and_back(&mut h, &EN.learn.backups);

    h.tap(ids::DETAIL_BACKUP);
    assert_eq!(h.app.screen(), ScreenKind::BackupMenu);
    info_and_back(&mut h, &EN.learn.backups);

    h.open_wallets();
    info_and_back(&mut h, &EN.learn.wallet_kinds);

    h.tap(ids::WALLETS_ADD);
    assert_eq!(h.app.screen(), ScreenKind::AddWallet);
    info_and_back(&mut h, &EN.learn.wallet_kinds);

    h.open_tile(TILE_TOOLS);
    info_and_back(&mut h, &EN.learn.tools);

    h.open_settings();
    info_and_back(&mut h, &EN.learn.devices);

    h.tap(ids::SETTINGS_ABOUT_ROW);
    assert_eq!(h.app.screen(), ScreenKind::About);
    info_and_back(&mut h, &EN.learn.devices);

    h.open_tile(TILE_SCAN);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    info_and_back(&mut h, &EN.learn.air_gap);
}

/// A wallet's own screens: the page is the kinds of wallets, its
/// addresses are about verifying one, and its export is about the xpubs
/// that leave the device.
#[test]
fn a_wallets_screens_open_the_pages_they_are_about() {
    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    h.open_single_sig(0);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    info_and_back(&mut h, &EN.learn.wallet_kinds);

    h.tap(ids::WALLET_KEYS);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys);
    info_and_back(&mut h, &EN.learn.wallet_kinds);
    h.tap(ids::BACK);

    h.tap(ids::WALLET_ADDRESSES);
    assert_eq!(h.app.screen(), ScreenKind::Addresses);
    info_and_back(&mut h, &EN.learn.verifying);
    h.tap(ids::BACK);

    h.tap(ids::WALLET_EXPORT);
    assert_eq!(h.app.screen(), ScreenKind::Export);
    info_and_back(&mut h, &EN.learn.xpubs);
}

/// A single-sig wallet's page opens "Kinds of wallets" at the single-sig
/// section, not at the top of the page.
#[test]
fn a_wallet_opens_its_page_at_the_kind_it_is() {
    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    h.open_single_sig(0);
    open_info(&mut h, &EN.learn.wallet_kinds);

    let heading = h.app.rect_of(ids::LEARN_HEADING).expect("the section");
    let view = h.app.rect_of(ids::SCROLL).expect("the scrolling body");
    assert!(
        heading.y <= view.y + 1,
        "the page opened at {heading:?} and not at the top of {view:?}"
    );
    // The section it opened at is the one about wallets over one key.
    assert_eq!(
        EN.learn.wallet_kinds.sections[1].heading,
        h.app
            .texts()
            .into_iter()
            .find(|t| *t == EN.learn.wallet_kinds.sections[1].heading)
            .unwrap_or_default()
    );
}

/// Create's source Choice is about randomness; the step that rolls it is
/// about where randomness comes from.
#[test]
fn the_create_wizard_opens_the_page_of_its_step() {
    let mut h = Harness::new(PANEL);
    h.open_create();
    assert_eq!(h.app.create_step(), Some(create::Step::Source));
    info_and_back(&mut h, &EN.learn.randomness);
    assert_eq!(h.app.create_step(), Some(create::Step::Source));

    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, 0),
        ids::CREATE_SOURCE_CONTINUE,
    );
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 1),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
    // The dice procedure Choice is part of the same question and opens
    // the same page.
    assert_eq!(h.app.create_step(), Some(create::Step::Procedure));
    info_and_back(&mut h, &EN.learn.where_randomness);
    h.tap(ids::CREATE_PROCEDURE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(create::Step::Entropy));
    info_and_back(&mut h, &EN.learn.where_randomness);
    assert_eq!(h.app.create_step(), Some(create::Step::Entropy));
}

/// Load's source Choice is about what the checked row loads: words,
/// an encrypted backup, or Seed XOR parts.
#[test]
fn the_load_source_opens_the_page_of_the_checked_row() {
    let sources = [
        (ids::LOAD_SOURCE_TYPE, &EN.learn.words),
        (ids::LOAD_SOURCE_BACKUP, &EN.learn.encrypted_backups),
        (ids::LOAD_SOURCE_XOR, &EN.learn.seed_xor),
        (ids::LOAD_SOURCE_SLIP39, &EN.learn.other_backups),
        (ids::LOAD_SOURCE_CODEX32, &EN.learn.other_backups),
    ];
    for (row, page) in sources {
        let mut h = Harness::new(PANEL);
        h.open_load();
        h.tap(row);
        assert_eq!(h.app.load_step(), Some(LoadStep::Source));
        info_and_back(&mut h, page);
        assert_eq!(h.app.load_step(), Some(LoadStep::Source));
    }
}

/// Every step of Load a key › SLIP-39 shares opens "Backups in other
/// forms" at the SLIP-39 section, and the chevron comes back to the step
/// with the shares already typed still in hand.
#[test]
fn the_slip39_steps_open_the_slip39_section() {
    const SHARE: &str = "shadow pistol academic always adequate wildlife fancy gross oasis \
                         cylinder mustang wrist rescue view short owner flip making coding armed";
    let section = EN.learn.other_backups.sections[1].heading;
    let mut h = Harness::new(PANEL);
    h.open_load();
    h.tap(ids::LOAD_SOURCE_SLIP39);
    open_info(&mut h, &EN.learn.other_backups);
    assert!(h.app.texts().iter().any(|t| t == section), "{section}");
    h.tap(ids::BACK);

    h.tap(ids::LOAD_SOURCE_CONTINUE);
    info_and_back(&mut h, &EN.learn.other_backups);

    h.tap(ids::LOAD_COUNT_CONTINUE);
    assert_eq!(h.app.load_step(), Some(LoadStep::Words));
    info_and_back(&mut h, &EN.learn.other_backups);

    h.type_share(SHARE);
    assert_eq!(h.app.load_step(), Some(LoadStep::Checksum));
    info_and_back(&mut h, &EN.learn.other_backups);
    assert_eq!(h.app.load_step(), Some(LoadStep::Checksum));
}

/// Every step of Load a key › Codex32 opens "Backups in other forms" at
/// the codex32 section, and the chevron comes back to the step with the
/// string already typed still in hand.
#[test]
fn the_codex32_steps_open_the_codex32_section() {
    const SECRET: &str = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";
    const SHARE: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
    let section = EN.learn.other_backups.sections[2].heading;
    let mut h = Harness::new(PANEL);
    h.open_load();
    h.tap(ids::LOAD_SOURCE_CODEX32);
    open_info(&mut h, &EN.learn.other_backups);
    assert!(h.app.texts().iter().any(|t| t == section), "{section}");
    h.tap(ids::BACK);

    h.tap(ids::LOAD_SOURCE_CONTINUE);
    assert_eq!(h.app.load_step(), Some(LoadStep::Words));
    info_and_back(&mut h, &EN.learn.other_backups);

    h.type_codex32(SHARE);
    assert_eq!(h.app.load_step(), Some(LoadStep::Checksum));
    info_and_back(&mut h, &EN.learn.other_backups);
    assert_eq!(h.app.load_step(), Some(LoadStep::Checksum));

    // The confirmation a secret lands on is a key's, not codex32's, so
    // it carries no button of its own for the form it was read from.
    let mut h = Harness::new(PANEL);
    h.open_codex32();
    h.type_codex32(SECRET);
    assert_eq!(h.app.load_step(), Some(LoadStep::Confirm));
}

/// The passphrase step of a wizard opens the passphrases page, and the
/// chevron comes back to the step with the words the wizard has already
/// taken still in it.
#[test]
fn the_page_leaves_the_wizard_where_it_was() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    assert_eq!(h.app.load_step(), Some(LoadStep::PassphraseOffer));
    assert_eq!(h.app.load_words_accepted(), ABANDON.len());

    open_info(&mut h, &EN.learn.passphrases);
    h.tap(ids::BACK);

    assert_eq!(h.app.screen(), ScreenKind::Load);
    assert_eq!(h.app.load_step(), Some(LoadStep::PassphraseOffer));
    assert_eq!(h.app.load_words_accepted(), ABANDON.len());
}

/// The two ways a key is opened from another: the passphrase screen is
/// about passphrases and the BIP-85 child is about the words it makes.
#[test]
fn the_screens_that_open_a_key_from_a_key_carry_their_pages() {
    let mut h = Harness::new(PANEL);
    load_key(&mut h);

    h.open_passphrase(0);
    info_and_back(&mut h, &EN.learn.passphrases);

    h.go_home();
    h.open_child(0);
    info_and_back(&mut h, &EN.learn.words);
}

/// Add a wallet's kind Choice is about the kinds; a FROST group's counts
/// are about FROST.
#[test]
fn the_wallet_wizard_opens_the_kinds_and_then_the_kind() {
    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    h.open_add_wallet(ids::BUILD_NEW);
    assert_eq!(h.app.screen(), ScreenKind::Build);
    info_and_back(&mut h, &EN.learn.wallet_kinds);

    h.choose(ids::at(ids::BUILD_KIND_BASE, 4), ids::BUILD_KIND_CONTINUE);
    info_and_back(&mut h, &EN.learn.frost);
}

/// Each kind's own steps open "Kinds of wallets" at that kind's section:
/// the two kinds this pass added carry their own, and a recovery
/// wallet's Later, threshold and wait steps carry the same one.
#[test]
fn the_new_kinds_open_their_own_sections() {
    for (kind, section) in [
        (2usize, "Taproot multisig"),
        (5, "Recovery and inheritance"),
    ] {
        let mut h = Harness::new(PANEL);
        load_key(&mut h);
        h.open_add_wallet(ids::BUILD_NEW);
        h.choose(
            ids::at(ids::BUILD_KIND_BASE, kind),
            ids::BUILD_KIND_CONTINUE,
        );
        open_info(&mut h, &EN.learn.wallet_kinds);
        assert!(
            h.app.texts().iter().any(|t| t == section),
            "{section} is not the section it opened: {:?}",
            h.app.texts()
        );
        h.tap(ids::BACK);
        assert_eq!(h.app.screen(), ScreenKind::Build);
    }
}

/// A recovery wallet's later steps stay on the kind's section all the
/// way to the review: the keys that spend later, the wait, the pad a
/// wait is typed on, and the question that offers a further path.
#[test]
fn the_recovery_steps_stay_on_the_recovery_section() {
    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    load_second_key(&mut h);
    h.open_add_wallet(ids::BUILD_NEW);
    h.choose(ids::at(ids::BUILD_KIND_BASE, 5), ids::BUILD_KIND_CONTINUE);
    h.tap(ids::at(ids::BUILD_WHICH_BASE, 0));
    h.tap(ids::BUILD_CONTINUE);
    info_and_back(&mut h, &EN.learn.wallet_kinds);

    h.tap(ids::at(ids::BUILD_WHICH_BASE, 1));
    h.tap(ids::BUILD_LATER_CONTINUE);
    info_and_back(&mut h, &EN.learn.wallet_kinds);

    h.tap(ids::BUILD_DELAY_TYPE);
    h.tap(ids::BUILD_DELAY_CONTINUE);
    info_and_back(&mut h, &EN.learn.wallet_kinds);

    h.tap(ids::BACK);
    h.tap(ids::at(ids::BUILD_DELAY_BASE, 0));
    h.tap(ids::BUILD_DELAY_CONTINUE);
    info_and_back(&mut h, &EN.learn.wallet_kinds);
}

/// A screen with a secret keeps the eye in the slot and takes no info
/// button: one control per screen, and the eye is the one that matters
/// where a secret is on the panel.
#[test]
fn a_screen_with_the_eye_takes_no_button() {
    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_WORDS);
    assert_eq!(h.app.screen(), ScreenKind::Backup);
    assert!(h.app.rect_of(ids::SECRET_EYE).is_some(), "the eye is gone");
    assert!(
        h.app.rect_of(ids::INFO).is_none(),
        "the words screen carries two controls"
    );
}

/// Home, Learn and the lock screen explain nothing and lead nowhere:
/// their trailing slot is empty.
#[test]
fn home_learn_and_the_lock_screen_leave_the_slot_empty() {
    let mut h = Harness::new(PANEL);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(h.app.rect_of(ids::INFO).is_none(), "Home explains itself");

    h.open_tile(TILE_LEARN);
    assert_eq!(h.app.screen(), ScreenKind::Learn);
    assert!(h.app.rect_of(ids::INFO).is_none(), "Learn leads to Learn");

    h.tap(ids::at(ids::LEARN_ROW_BASE, 0));
    assert_eq!(h.app.screen(), ScreenKind::LearnPage);
    assert!(h.app.rect_of(ids::INFO).is_none(), "a page leads to itself");

    h.go_home();
    load_key(&mut h);
    h.send(Event::Lock);
    assert_eq!(h.app.screen(), ScreenKind::Lock);
    assert!(
        h.app.rect_of(ids::INFO).is_none(),
        "the lock screen offers a way out"
    );
}

/// A page reached from the Learn menu still ends in the row that opens
/// the flow it is about (§16.85); only a page opened from a working
/// screen leaves it off.
#[test]
fn the_learn_menu_still_offers_the_flow_a_page_is_about() {
    let words = EN
        .learn
        .pages()
        .iter()
        .position(|p| core::ptr::eq(*p, &EN.learn.words))
        .expect("the words page is listed");
    let mut h = Harness::new(PANEL);
    h.open_tile(TILE_LEARN);
    h.tap(ids::at(ids::LEARN_ROW_BASE, words));
    assert_eq!(h.app.screen(), ScreenKind::LearnPage);
    assert!(
        h.app.reveal(ids::LEARN_TRY).is_some(),
        "the page lost its Try it row"
    );
}

/// The Sign review is about transactions, at every step of the review
/// and at the hold, and the chevron comes back to the step it was on
/// with the transaction still loaded.
#[test]
fn the_sign_review_opens_the_transactions_page() {
    const DEMO: &[u8] = include_bytes!("../../../tools/vectors/psbt/demo-regtest.psbt");
    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    h.set_network(3);
    h.open_single_sig(0);
    h.tap(ids::WALLET_SIGN);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Summary)));
    info_and_back(&mut h, &EN.learn.transactions);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Summary)));

    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Outputs)));
    info_and_back(&mut h, &EN.learn.transactions);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Outputs)));
}

/// "Check an address" is about verifying one.
#[test]
fn checking_an_address_opens_the_verifying_page() {
    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    h.open_single_sig(0);
    h.tap(ids::WALLET_CHECK);
    // Scan is the way in for anything read, and it is about the air gap.
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    info_and_back(&mut h, &EN.learn.air_gap);
    h.tap(ids::SCAN_TYPE);
    assert_eq!(h.app.screen(), ScreenKind::Verify);
    info_and_back(&mut h, &EN.learn.verifying);
}

/// Writing a SLIP-39 backup: the word count a share has, every Choice
/// of the plan and the result behind them are about the forms a backup
/// is written in.
#[test]
fn writing_slip39_shares_opens_the_other_backups_page() {
    use opensigner_core::shares::Step as ShareStep;

    let mut h = Harness::new(PANEL);
    h.open_add(ids::ADD_CREATE_SLIP39);
    // The source Choice is about randomness, as Create a key's is.
    info_and_back(&mut h, &EN.learn.randomness);
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, 6),
        ids::CREATE_SOURCE_CONTINUE,
    );
    assert_eq!(h.app.create_step(), Some(create::Step::Count));
    info_and_back(&mut h, &EN.learn.other_backups);
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.tap(ids::CREATE_CONTINUE);
    // The passphrase steps are about passphrases wherever they are.
    info_and_back(&mut h, &EN.learn.passphrases);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    assert_eq!(h.app.split_step(), Some(ShareStep::Groups));
    info_and_back(&mut h, &EN.learn.other_backups);
    h.choose(
        ids::at(ids::SHARE_GROUPS_BASE, 0),
        ids::SHARE_GROUPS_CONTINUE,
    );
    assert_eq!(h.app.split_step(), Some(ShareStep::Count));
    info_and_back(&mut h, &EN.learn.other_backups);
    h.choose(ids::at(ids::SHARE_COUNT_BASE, 1), ids::SHARE_COUNT_CONTINUE);
    assert_eq!(h.app.split_step(), Some(ShareStep::Threshold));
    info_and_back(&mut h, &EN.learn.other_backups);
}

/// Writing a codex32 backup: the seed length, every Choice of the plan
/// and the Secret screens behind them are about the forms a backup is
/// written in.
#[test]
fn writing_codex32_strings_opens_the_other_backups_page() {
    use opensigner_core::codex32::Step as PlanStep;

    let section = EN.learn.other_backups.sections[2].heading;
    let mut h = Harness::new(PANEL);
    h.open_add(ids::ADD_CREATE_CODEX32);
    // The source Choice is about randomness, as Create a key's is.
    info_and_back(&mut h, &EN.learn.randomness);
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, 6),
        ids::CREATE_SOURCE_CONTINUE,
    );
    assert_eq!(h.app.create_step(), Some(create::Step::Count));
    open_info(&mut h, &EN.learn.other_backups);
    assert!(h.app.texts().iter().any(|t| t == section), "{section}");
    h.tap(ids::BACK);
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Split));
    info_and_back(&mut h, &EN.learn.other_backups);
    h.choose(ids::CODEX32_SPLIT_YES, ids::CODEX32_SPLIT_CONTINUE);
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Count));
    info_and_back(&mut h, &EN.learn.other_backups);
    h.choose(
        ids::at(ids::CODEX32_COUNT_BASE, 1),
        ids::CODEX32_COUNT_CONTINUE,
    );
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Threshold));
    info_and_back(&mut h, &EN.learn.other_backups);
}

/// Backup › Codex32 on a key made of words opens the same page: the
/// plan and the randomness are about the form, wherever they are asked.
#[test]
fn a_codex32_backup_opens_the_other_backups_page() {
    use opensigner_core::codex32::Step as PlanStep;

    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_CODEX32);
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Split));
    info_and_back(&mut h, &EN.learn.other_backups);
    h.choose(ids::CODEX32_SPLIT_YES, ids::CODEX32_SPLIT_CONTINUE);
    assert_eq!(h.app.codex32_step(), Some(PlanStep::Count));
    info_and_back(&mut h, &EN.learn.other_backups);
}

/// A key's account export is about what an xpub shows; BIP 129's key
/// record and the two values it is written from are about the files a
/// coordinator and a signer exchange (`docs/PLANNING.md` §16.110).
#[test]
fn a_keys_account_opens_xpubs_and_its_key_record_opens_coordinator_files() {
    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    h.open_key(0);
    h.tap(ids::DETAIL_ACCOUNT);
    // The SegWit multisig account, which is the one BIP 129 covers.
    h.choose(ids::at(ids::PICK_BASE, 5), ids::PICK_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Export);
    info_and_back(&mut h, &EN.learn.xpubs);

    let record = opensigner_core::ExportFormat::ALL
        .iter()
        .position(|f| *f == opensigner_core::ExportFormat::BsmsSigner)
        .expect("the format is in the list");
    h.tap(ids::EXPORT_FORMAT);
    h.choose(ids::at(ids::PICK_BASE, record), ids::PICK_CONTINUE);
    info_and_back(&mut h, &EN.learn.coordinators);

    h.tap(ids::BSMS_TOKEN_NONE);
    h.pad(
        ids::BSMS_TOKEN_KEYBOARD,
        osk_ui::widgets::keyboard::KeyInput::Done,
    );
    info_and_back(&mut h, &EN.learn.coordinators);
    h.pad(
        ids::BSMS_DESCRIPTION_KEYBOARD,
        osk_ui::widgets::keyboard::KeyInput::Done,
    );
    assert_eq!(h.app.screen(), ScreenKind::Export);
    info_and_back(&mut h, &EN.learn.coordinators);
}

/// The keys a transaction names are about transactions, as the review
/// they were opened from is.
#[test]
fn the_transactions_keys_review_opens_transactions() {
    const DEMO: &[u8] = include_bytes!("../../../tools/vectors/psbt/demo-regtest.psbt");
    let mut h = Harness::new(common::PHONE);
    load_key(&mut h);
    h.set_network(3);
    h.open_single_sig(0);
    h.tap(ids::WALLET_SIGN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Sign);
    h.tap(ids::SIGN_KEYS);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys);
    info_and_back(&mut h, &EN.learn.transactions);
}

/// The page that says what a signature was checked for, and the tool
/// that compares two transactions, are both about nonces
/// (`docs/PLANNING.md` §16.111).
#[test]
fn the_signatures_page_and_the_compare_tool_open_the_nonces_page() {
    const DEMO: &[u8] = include_bytes!("../../../tools/vectors/psbt/demo-regtest.psbt");
    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    h.set_network(3);
    h.open_single_sig(0);
    h.tap(ids::WALLET_SIGN);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    // Through the review to the hold, and from the result to the page
    // that lists what the transaction now carries.
    for _ in 0..4 {
        h.tap(ids::SIGN_CONTINUE);
    }
    h.hold(ids::SIGN_HOLD);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Result)));
    h.tap(ids::SIGN_SIGNATURES);
    assert_eq!(
        h.app.sign_stage(),
        Some(Stage::Wizard(SignStep::Signatures))
    );
    info_and_back(&mut h, &EN.learn.nonces);

    // Compare transactions, from its first transaction on.
    let mut h = Harness::new(PANEL);
    h.set_network(3);
    h.go_home();
    h.open_tile(TILE_TOOLS);
    h.tap(ids::TOOLS_COMPARE);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::CompareTx);
    info_and_back(&mut h, &EN.learn.nonces);
}

/// BIP-85's other applications are explained where BIP-85 is, and
/// Bitcoin Core's import file is a coordinator's file
/// (`docs/PLANNING.md` §16.114).
#[test]
fn the_bip85_pads_open_seed_words_and_the_core_import_opens_coordinator_files() {
    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    h.open_key(0);
    h.tap(ids::DETAIL_BIP85);
    // A base64 password, which asks its length and then its index.
    h.choose(ids::at(ids::PICK_BASE, 4), ids::PICK_CONTINUE);
    info_and_back(&mut h, &EN.learn.words);
    h.pad(
        ids::BIP85_LENGTH_PAD,
        osk_ui::widgets::keyboard::KeyInput::Done,
    );
    info_and_back(&mut h, &EN.learn.words);
    // The Secret screen the index opens carries the eye in that slot,
    // so there is no button on it.
    h.pad(
        ids::BIP85_INDEX_PAD,
        osk_ui::widgets::keyboard::KeyInput::Done,
    );
    assert!(h.app.rect_of(ids::INFO).is_none(), "the derived value");

    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    h.set_network(3);
    h.add_single_sig(0, 2);
    h.tap(ids::WALLET_EXPORT);
    info_and_back(&mut h, &EN.learn.xpubs);
    let import = opensigner_core::ExportFormat::ALL
        .iter()
        .position(|f| *f == opensigner_core::ExportFormat::CoreImport)
        .expect("the format is in the list");
    h.tap(ids::EXPORT_FORMAT);
    h.choose(ids::at(ids::PICK_BASE, import), ids::PICK_CONTINUE);
    h.choose(ids::at(ids::PICK_BASE, 1), ids::PICK_CONTINUE);
    info_and_back(&mut h, &EN.learn.coordinators);
}

/// A silent payments wallet's own screens are about silent payments,
/// and its page opens "Kinds of wallets" at the section for the kind it
/// is (`docs/PLANNING.md` §16.113).
#[test]
fn a_silent_wallets_screens_open_the_silent_payments_page() {
    let mut h = Harness::new(PANEL);
    load_key(&mut h);
    let silent = opensigner_core::build::WalletKind::ALL
        .iter()
        .position(|k| *k == opensigner_core::build::WalletKind::Silent)
        .expect("the kind is in the list");
    h.open_add_wallet(ids::BUILD_NEW);
    h.choose(
        ids::at(ids::BUILD_KIND_BASE, silent),
        ids::BUILD_KIND_CONTINUE,
    );
    h.tap(ids::at(ids::BUILD_WHICH_BASE, 0));
    h.tap(ids::BUILD_CONTINUE);
    h.tap(ids::BUILD_ADD_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    info_and_back(&mut h, &EN.learn.wallet_kinds);

    h.tap(ids::SILENT_ADDRESS);
    assert_eq!(h.app.screen(), ScreenKind::SilentAddress);
    info_and_back(&mut h, &EN.learn.silent_payments);
    h.tap(ids::BACK);

    h.tap(ids::SILENT_LABELS);
    assert_eq!(h.app.screen(), ScreenKind::SilentLabels);
    info_and_back(&mut h, &EN.learn.silent_payments);
    h.tap(ids::BACK);

    h.tap(ids::SILENT_CHECK);
    assert_eq!(h.app.screen(), ScreenKind::SilentCheck);
    info_and_back(&mut h, &EN.learn.silent_payments);
    h.tap(ids::BACK);

    // The export is about the xpubs that leave the device, as every
    // export is; the Secret screen behind it carries the eye in that
    // slot, so there is no button on it.
    h.tap(ids::WALLET_EXPORT);
    info_and_back(&mut h, &EN.learn.xpubs);
    h.tap(ids::SILENT_SECRET_ROW);
    assert_eq!(h.app.screen(), ScreenKind::SilentSecret);
    assert!(h.app.rect_of(ids::INFO).is_none(), "the scan descriptor");
}
