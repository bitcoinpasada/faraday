//! Decode a transaction and Check a signed message open from Tools on what
//! waits in Files: the file itself when one does, a list to choose from
//! when several do, Scan, Open Files and the stick when none, and each
//! result ends with Another and Done.

use crate::common::{has, offered, through_fade};
use faraday_core::core_wallet::tool::CoreWalletAction as W;
use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// The first transaction ever sent, as the hex of its bytes.
const TX: &str = "0100000001c997a5e56e104102fa209c6a852dd90660a20b2d9c352423edce25857fcd3704000000004847304402204e45e16932b8af514961a1d3a1a25fdf3f4f7732e9d624c6c61548ab5fb8cd410220181522ec8eca07de4860a4acdd12909d831cc56cbbac4622082221a8768d1d0901ffffffff0200ca9a3b00000000434104ae1a62fe09c5f51b13905f07f06b99a2f7159b2225f374cd378d71302fa28414e7aab37397f554a7df5f142c21c1b7303b8a0626f1baded5c72a704f7e6cd84cac00286bee0000000043410411db93e1dcdb8a016b49840f8c53bc1eb68a382e97b1482ecad7b148a6909a5cb2e0eaddfb84ccf9744464f82e160bfa9b8b64f9d4c03f999b8643f656b412a3ac00000000";

/// A signed message in the three-line form.
const MESSAGE: &str = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq\nH9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=\nHello\n";

fn tools_with(files: &[(&str, &[u8])]) -> Faraday {
    let mut app = testkit::started();
    // The signed message and the transaction are Mainnet's.
    testkit::move_to(&mut app.session, faraday_bip::keys::Network::Mainnet).unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(
            files
                .iter()
                .map(|(n, b)| (faraday_text::Text::of(n), b.to_vec()))
                .collect(),
        ),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Catalog));
    app
}

fn open_tile(app: &mut Faraday, name: &str) {
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == name)
        .unwrap();
    app.press(Action::Catalog(tile as u8));
}

#[test]
fn decode_with_two_transactions_lists_both_and_decodes_the_chosen() {
    let mut app = tools_with(&[("first.txt", TX.as_bytes()), ("second.txt", TX.as_bytes())]);
    on_wide(&mut app);
    open_tile(&mut app, EN.catalog_decode_a_transaction);
    assert_eq!(app.screen, Screen::Decode);
    assert!(app.decode().is_none());
    assert!(has(&mut app, "first.txt"));
    assert!(has(&mut app, "second.txt"));
    app.press(Action::DecodeInbox(1));
    through_fade(&mut app);
    let d = app.decode().expect("the chosen one is decoded");
    assert!(d.decoded.source.contains("second.txt"));
    // Another shows the list again; Done goes back to Tools.
    assert!(has(&mut app, EN.bip85_another));
    assert!(has(&mut app, EN.app_done));
    app.press(Action::DecodeAnother);
    through_fade(&mut app);
    assert!(app.decode().is_none());
    assert!(has(&mut app, "first.txt"));
    app.press(Action::DecodeInbox(0));
    assert_eq!(app.decode().unwrap().back, Screen::Catalog);
}

#[test]
fn decode_with_one_transaction_opens_it_and_with_none_offers_the_ways_in() {
    let mut one = tools_with(&[("only.txt", TX.as_bytes())]);
    open_tile(&mut one, EN.catalog_decode_a_transaction);
    assert!(one.decode().is_some());

    let mut none = tools_with(&[]);
    open_tile(&mut none, EN.catalog_decode_a_transaction);
    assert_eq!(none.screen, Screen::Decode);
    assert!(has(&mut none, EN.decode_no_transaction_waits));
    assert!(has(&mut none, EN.compact_scan));
    assert!(has(&mut none, EN.parts_open_files));
}

#[test]
fn check_message_lists_several_and_offers_the_ways_in_with_none() {
    let mut two = tools_with(&[("a.txt", MESSAGE.as_bytes()), ("b.txt", MESSAGE.as_bytes())]);
    open_tile(&mut two, EN.catalog_check_a_signed);
    assert_eq!(two.screen, Screen::CheckMessage);
    assert!(has(&mut two, "a.txt"));
    assert!(has(&mut two, "b.txt"));
    two.press(Action::CheckMessage(1));
    assert!(has(&mut two, "b.txt"));
    assert!(has(&mut two, EN.bip85_another));
    assert!(has(&mut two, EN.app_done));
    two.press(Action::CheckMessageAnother);
    assert!(has(&mut two, "a.txt"));

    let mut one = tools_with(&[("only.txt", MESSAGE.as_bytes())]);
    open_tile(&mut one, EN.catalog_check_a_signed);
    assert!(has(&mut one, "only.txt"));
    assert!(has(&mut one, EN.check_file));

    // Nothing waits (with none, Practice opens its sample message, so the
    // one file is taken out after Another).
    one.press(Action::CheckMessageAnother);
    one.press(Action::InboxRemove(0));
    assert!(has(&mut one, EN.check_no_signed_message));
    assert!(has(&mut one, EN.compact_scan));
    assert!(has(&mut one, EN.parts_open_files));
}

// Check a multisig setup.

/// A wallet of three keys, as a file's text.
fn setup() -> String {
    let d = testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .unwrap()
        .descriptor;
    d.split('#').next().unwrap().to_string()
}

#[test]
fn check_multisig_opens_the_one_setup_and_lists_several_and_offers_the_ways_in_with_none() {
    let mut one = tools_with(&[("only.txt", setup().as_bytes())]);
    open_tile(&mut one, EN.mscheck_title);
    assert_eq!(one.screen, Screen::MultisigCheck);
    assert!(has(&mut one, EN.mscheck_add));
    // The ending is under the keys: scroll to it, the pointer over the
    // card (Your choices stands at its left and has nothing to scroll).
    for _ in 0..40 {
        if has(&mut one, EN.mscheck_done) {
            break;
        }
        one.event(faraday_shell_api::Event::Scroll {
            x: 800,
            y: 400,
            dy: 120,
        });
    }
    assert!(has(&mut one, EN.mscheck_another));
    assert!(has(&mut one, EN.mscheck_done));

    let mut two = tools_with(&[("a.txt", setup().as_bytes()), ("b.txt", setup().as_bytes())]);
    open_tile(&mut two, EN.mscheck_title);
    assert!(has(&mut two, EN.mscheck_choose_setup));
    assert!(has(&mut two, "a.txt"));
    assert!(has(&mut two, "b.txt"));
    assert!(!has(&mut two, EN.mscheck_add));

    let mut none = tools_with(&[]);
    open_tile(&mut none, EN.mscheck_title);
    assert!(has(&mut none, EN.mscheck_nothing_waits));
    assert!(has(&mut none, EN.compact_scan));
    assert!(has(&mut none, EN.parts_open_files));
}

// ----- Check an address -----

const ADDRESS: &str = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq";

#[test]
fn check_address_with_none_offers_the_ways_in_and_with_files_opens_or_lists_them() {
    let mut none = tools_with(&[("photo.bin", &[0xff, 0xfe, 0x00, 0x80])]);
    open_tile(&mut none, EN.catalog_check_an_address);
    assert_eq!(none.screen, Screen::CheckAddress);
    assert!(has(&mut none, EN.address_check_no_address));
    assert!(has(&mut none, EN.compact_scan));
    assert!(has(&mut none, EN.parts_open_files));

    // One text file is read at once: the address in it is checked.
    let mut one = tools_with(&[("only.txt", ADDRESS.as_bytes())]);
    open_tile(&mut one, EN.catalog_check_an_address);
    assert_eq!(one.address_check_state().unwrap().typed().as_str(), ADDRESS);

    // Several: the files to choose from, then the chosen one is read.
    let mut two = tools_with(&[("a.txt", ADDRESS.as_bytes()), ("b.txt", ADDRESS.as_bytes())]);
    open_tile(&mut two, EN.catalog_check_an_address);
    assert!(has(&mut two, "a.txt"));
    assert!(has(&mut two, "b.txt"));
    assert!(two.address_check_state().unwrap().typed().is_empty());
    two.press(Action::AddressCheck(
        faraday_core::address_check::AddressPress::File(1),
    ));
    assert_eq!(two.address_check_state().unwrap().typed().as_str(), ADDRESS);
}

#[test]
fn check_address_with_nothing_loaded_offers_to_add_a_key_or_load_a_wallet() {
    let mut app = tools_with(&[("only.txt", ADDRESS.as_bytes())]);
    open_tile(&mut app, EN.catalog_check_an_address);
    assert!(app.address_check_state().unwrap().load_first());
    assert!(has(&mut app, EN.address_check_load_first));
    assert!(has(&mut app, EN.catalog_add_a_key));
    assert!(has(&mut app, EN.decode_load_a_wallet));
    app.press(Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry, "Add a key opens its page");

    let mut app = tools_with(&[("only.txt", ADDRESS.as_bytes())]);
    open_tile(&mut app, EN.catalog_check_an_address);
    app.press(Action::RestoreFor(faraday_core::restore::Check::Address));
    assert_eq!(
        app.screen,
        Screen::Restore,
        "the wallet is loaded in Restore"
    );
}

#[test]
fn check_address_result_ends_with_another_and_done() {
    let descriptor = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84h/1h/0h"));
    let mut app = tools_with(&[
        ("wallet.txt", descriptor.as_bytes()),
        ("only.txt", b"tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx"),
    ]);
    app.press(Action::LoadWallet(0));
    app.press(Action::Nav(Screen::Catalog));
    open_tile(&mut app, EN.catalog_check_an_address);
    assert!(has(&mut app, EN.bip85_another));
    assert!(has(&mut app, EN.app_done));
    app.press(Action::AddressCheck(
        faraday_core::address_check::AddressPress::Another,
    ));
    assert!(app.address_check_state().unwrap().typed().is_empty());
    assert!(has(&mut app, "only.txt"));
}

// ----- Read a Bitcoin Core wallet -----

fn wallet_vector(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../tools/vectors/core-wallet/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn core_wallet_lists_several_files_and_ends_with_another_and_done() {
    let (plain, enc) = (wallet_vector("plain.dat"), wallet_vector("encrypted.dat"));
    let mut app = tools_with(&[("one.dat", &plain), ("two.dat", &enc)]);
    app.session = faraday_core::wallet::Session::on(faraday_bip::keys::Network::Testnet);
    open_tile(&mut app, EN.catalog_read_core_wallet);
    assert_eq!(app.screen, Screen::CoreWallet);
    assert!(has(&mut app, "one.dat"));
    assert!(has(&mut app, "two.dat"));
    assert!(!has(&mut app, EN.core_wallet_read));
    app.press(Action::CoreWallet(W::File(0)));
    app.press(Action::CoreWallet(W::Read));
    app.event(faraday_shell_api::Event::Tick { now_ms: 1 });
    crate::common::through_fade(&mut app);
    assert!(has(&mut app, EN.core_wallet_whole));
    // The ending is under the result: scroll to it.
    for _ in 0..40 {
        if has(&mut app, EN.bip85_another) {
            break;
        }
        app.event(faraday_shell_api::Event::Scroll {
            x: 240,
            y: 400,
            dy: 120,
        });
    }
    assert!(has(&mut app, EN.bip85_another));
    assert!(has(&mut app, EN.app_done));
    app.press(Action::CoreWallet(W::Another));
    assert!(has(&mut app, "two.dat"));
    assert!(!has(&mut app, EN.core_wallet_whole));
}

#[test]
fn core_wallet_with_one_file_opens_it_and_with_none_offers_the_ways_in() {
    let plain = wallet_vector("plain.dat");
    let mut one = tools_with(&[("only.dat", &plain)]);
    open_tile(&mut one, EN.catalog_read_core_wallet);
    assert!(has(&mut one, EN.core_wallet_read));

    let mut none = tools_with(&[]);
    open_tile(&mut none, EN.catalog_read_core_wallet);
    assert_eq!(none.screen, Screen::CoreWallet);
    // Practice puts its sample wallet in Files; take it out for the empty part.
    none.press(Action::CoreWallet(W::Another));
    while !none.inbox().is_empty() {
        none.press(Action::InboxRemove(0));
    }
    assert!(has(&mut none, EN.core_wallet_no_file_waits));
    // A wallet file cannot come by QR: Open Files, and no Scan in the card
    // (the sidebar's is the one Scan drawn).
    let _ = none.frame();
    let scans = none
        .drawn_texts()
        .iter()
        .filter(|t| *t == EN.compact_scan)
        .count();
    assert!(scans <= 1, "{scans} Scan buttons");
    assert!(has(&mut none, EN.parts_open_files));
}

// Check a timestamp

const OTS_V: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/vectors/opentimestamps"
);

fn proof() -> Vec<u8> {
    std::fs::read(format!("{OTS_V}/examples/different-blockchains.txt.ots")).unwrap()
}

#[test]
fn timestamp_with_one_proof_opens_it_and_with_several_lists_them() {
    let mut one = tools_with(&[("only.ots", &proof())]);
    open_tile(&mut one, EN.catalog_check_timestamp);
    assert_eq!(one.screen, Screen::OtsCheck);
    assert!(has(&mut one, "only.ots"));
    assert!(!has(&mut one, EN.ots_choose_proof));

    let mut two = tools_with(&[("a.ots", &proof()), ("b.ots", &proof())]);
    open_tile(&mut two, EN.catalog_check_timestamp);
    assert!(has(&mut two, EN.ots_choose_proof));
    assert!(has(&mut two, "a.ots"));
    assert!(has(&mut two, "b.ots"));
    two.press(Action::OtsCheck(faraday_core::ots_check::OtsAction::Proof(
        1,
    )));
    assert!(!has(&mut two, EN.ots_choose_proof));
    assert!(has(&mut two, "b.ots"));
}

#[test]
fn timestamp_with_no_proof_offers_the_ways_in() {
    let mut none = tools_with(&[("notes.txt", b"hello")]);
    open_tile(&mut none, EN.catalog_check_timestamp);
    // Practice's sample stands in; take it out to see the empty part.
    if !has(&mut none, EN.ots_no_proof_waits) {
        none.press(Action::OtsCheck(
            faraday_core::ots_check::OtsAction::Another,
        ));
    }
    assert!(has(&mut none, EN.ots_no_proof_waits));
    assert!(has(&mut none, EN.compact_scan));
    assert!(has(&mut none, EN.parts_open_files));
}

// Explore, BIP-85, Vanity and Silent payments with no key loaded.

/// What opens each of the four tools.
fn key_tools() -> [(Action, Screen); 4] {
    [
        (Action::Explore, Screen::Explore),
        (Action::Bip85, Screen::Bip85),
        (
            Action::Vanity(faraday_core::vanity::VanityAction::Open),
            Screen::Vanity,
        ),
        (Action::Silent, Screen::Silent),
    ]
}

#[test]
fn key_tools_open_with_no_key_on_add_a_key_and_say_nothing_else() {
    for (open, screen) in key_tools() {
        let mut app = testkit::started();
        assert!(app.session.keys.is_empty());
        app.press(Action::Nav(Screen::Catalog));
        app.press(open);
        assert_eq!(app.screen, screen);
        assert!(has(&mut app, EN.convert_add_a_key), "{screen:?}");
        assert!(!has(&mut app, "Load a key first"), "{screen:?}");
    }
}

#[test]
fn add_a_key_from_a_key_tool_comes_back_to_it() {
    for (open, screen) in key_tools() {
        let mut app = testkit::started();
        app.press(Action::Nav(Screen::Catalog));
        app.press(open);
        app.press(Action::KeyGen(None));
        assert_eq!(app.screen, Screen::KeyGen, "{screen:?}");
        app.press(Action::Back);
        assert_eq!(app.screen, screen, "{screen:?}");
        assert!(has(&mut app, EN.convert_add_a_key), "{screen:?}");
    }
}

/// The app on a wide window, as `tools_with` makes it.
fn on_wide(app: &mut Faraday) {
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 4000,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// Decode a transaction on a wide window: the pick holds the transactions
/// in Files and Scan, and a chosen one is the Result card with its
/// buttons, the pick still there.
#[test]
fn decode_on_a_wide_window_has_the_pick_and_the_result_card() {
    let mut app = tools_with(&[("first.txt", TX.as_bytes()), ("second.txt", TX.as_bytes())]);
    on_wide(&mut app);
    open_tile(&mut app, EN.catalog_decode_a_transaction);
    through_fade(&mut app);
    assert_eq!(app.screen, Screen::Decode);
    assert!(offered(&mut app, Action::DecodeInbox(0)));
    assert!(offered(&mut app, Action::DecodeInbox(1)));
    assert!(offered(
        &mut app,
        Action::ScanFor(faraday_core::Ask::Decode)
    ));

    app.press(Action::DecodeInbox(1));
    through_fade(&mut app);
    assert!(has(&mut app, EN.decode_transaction));
    assert!(has(&mut app, EN.decode_show_the_hex));
    assert!(has(&mut app, EN.bip85_another));
    // The pick stays at the left, and another file can be chosen from it.
    assert!(offered(&mut app, Action::DecodeInbox(0)));
    assert!(offered(
        &mut app,
        Action::ScanFor(faraday_core::Ask::Decode)
    ));
    app.press(Action::DecodeInbox(0));
    let d = app.decode().expect("the other file is decoded");
    assert!(d.decoded.source.contains("first.txt"));
}

/// The way back from the card goes where the tool was opened from.
#[test]
fn decode_on_a_wide_window_goes_back_to_where_it_was_opened_from() {
    let mut app = tools_with(&[("first.txt", TX.as_bytes()), ("second.txt", TX.as_bytes())]);
    on_wide(&mut app);
    open_tile(&mut app, EN.catalog_decode_a_transaction);
    app.press(Action::DecodeInbox(0));
    through_fade(&mut app);
    assert_eq!(app.decode().unwrap().back, Screen::Catalog);
    assert!(has(&mut app, EN.decode_back_tools));
    assert!(offered(&mut app, Action::Nav(Screen::Catalog)));
    app.press(Action::Nav(Screen::Catalog));
    assert_eq!(app.screen, Screen::Catalog);
}

/// Check a signed message on a wide window: the pick holds the signed
/// messages in Files, Scan and Open Files; a chosen one is the Result card
/// with the pick still at the left, and another can be chosen from it.
#[test]
fn check_message_on_a_wide_window_has_the_pick_and_the_result_card() {
    let mut app = tools_with(&[("a.txt", MESSAGE.as_bytes()), ("b.txt", MESSAGE.as_bytes())]);
    on_wide(&mut app);
    open_tile(&mut app, EN.catalog_check_a_signed);
    through_fade(&mut app);
    assert_eq!(app.screen, Screen::CheckMessage);
    assert!(offered(&mut app, Action::CheckMessage(0)));
    assert!(offered(&mut app, Action::CheckMessage(1)));
    assert!(offered(&mut app, Action::Scan));
    assert!(offered(&mut app, Action::Nav(Screen::Files)));

    app.press(Action::CheckMessage(1));
    through_fade(&mut app);
    assert!(has(&mut app, EN.check_file));
    assert!(has(&mut app, EN.check_address));
    assert!(has(&mut app, EN.bip85_another));
    // The pick stays at the left.
    assert!(offered(&mut app, Action::CheckMessage(0)));
    assert!(offered(&mut app, Action::Scan));
    app.press(Action::CheckMessage(0));
    through_fade(&mut app);
    assert!(has(&mut app, "a.txt"));
    // Another returns to the pick alone.
    app.press(Action::CheckMessageAnother);
    through_fade(&mut app);
    assert!(!has(&mut app, EN.check_address));
    assert!(offered(&mut app, Action::CheckMessage(1)));
}

/// With nothing held the pick on a wide window says so and offers the ways in.
#[test]
fn check_message_on_a_wide_window_with_none_offers_the_ways_in() {
    let mut app = tools_with(&[("only.txt", MESSAGE.as_bytes())]);
    on_wide(&mut app);
    open_tile(&mut app, EN.catalog_check_a_signed);
    app.press(Action::CheckMessageAnother);
    app.press(Action::InboxRemove(0));
    through_fade(&mut app);
    assert!(has(&mut app, EN.check_no_signed_message));
    assert!(offered(&mut app, Action::Scan));
    assert!(offered(&mut app, Action::Nav(Screen::Files)));
}

/// The way back from the card goes where the check was opened from: Tools,
/// or Files when a held file was pressed there.
#[test]
fn check_message_on_a_wide_window_goes_back_to_where_it_was_opened_from() {
    let mut app = tools_with(&[("a.txt", MESSAGE.as_bytes()), ("b.txt", MESSAGE.as_bytes())]);
    on_wide(&mut app);
    open_tile(&mut app, EN.catalog_check_a_signed);
    app.press(Action::CheckMessage(0));
    through_fade(&mut app);
    assert!(offered(&mut app, Action::Nav(Screen::Catalog)));
    app.press(Action::Nav(Screen::Catalog));
    assert_eq!(app.screen, Screen::Catalog);

    app.press(Action::Nav(Screen::Files));
    app.press(Action::CheckMessage(1));
    through_fade(&mut app);
    assert_eq!(app.screen, Screen::CheckMessage);
    assert!(offered(&mut app, Action::Nav(Screen::Files)));
    app.press(Action::Nav(Screen::Files));
    assert_eq!(app.screen, Screen::Files);
}
