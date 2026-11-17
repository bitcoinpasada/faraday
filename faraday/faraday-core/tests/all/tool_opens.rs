//! Decode a transaction and Check a signed message open from Tools on what
//! waits in Files: the file itself when one does, a list to choose from
//! when several do, Scan, Open Files and the stick when none, and each
//! result ends with Another and Done.

use crate::common::has;
use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};

/// The first transaction ever sent, as the hex of its bytes.
const TX: &str = "0100000001c997a5e56e104102fa209c6a852dd90660a20b2d9c352423edce25857fcd3704000000004847304402204e45e16932b8af514961a1d3a1a25fdf3f4f7732e9d624c6c61548ab5fb8cd410220181522ec8eca07de4860a4acdd12909d831cc56cbbac4622082221a8768d1d0901ffffffff0200ca9a3b00000000434104ae1a62fe09c5f51b13905f07f06b99a2f7159b2225f374cd378d71302fa28414e7aab37397f554a7df5f142c21c1b7303b8a0626f1baded5c72a704f7e6cd84cac00286bee0000000043410411db93e1dcdb8a016b49840f8c53bc1eb68a382e97b1482ecad7b148a6909a5cb2e0eaddfb84ccf9744464f82e160bfa9b8b64f9d4c03f999b8643f656b412a3ac00000000";

/// A signed message in the three-line form.
const MESSAGE: &str = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq\nH+dGVzdHNpZ25hdHVyZQ==\nHello\n";

fn tools_with(files: &[(&str, &[u8])]) -> Faraday {
    let mut app = testkit::started();
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
    open_tile(&mut app, EN.catalog_decode_a_transaction);
    assert_eq!(app.screen, Screen::Decode);
    assert!(app.decode().is_none());
    assert!(has(&mut app, "first.txt"));
    assert!(has(&mut app, "second.txt"));
    app.press(Action::DecodeInbox(1));
    let d = app.decode().expect("the chosen one is decoded");
    assert!(d.decoded.source.contains("second.txt"));
    // Another shows the list again; Done goes back to Tools.
    assert!(has(&mut app, EN.bip85_another));
    assert!(has(&mut app, EN.app_done));
    app.press(Action::DecodeAnother);
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
