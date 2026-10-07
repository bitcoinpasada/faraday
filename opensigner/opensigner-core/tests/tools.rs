//! Tools: the standalone calculators. Every row is live with nothing
//! loaded, because no tool needs a key.

mod common;

use common::{ABANDON, Harness, PANEL, PHONE, TINY};
use opensigner_core::tools::{self, KeyReading, ReadAs, Tool};
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::keys::Network;
use osk_bip::slip132;
use osk_psbt::Psbt;
use osk_shell_api::{Event, FileKind};
use osk_ui::components::{Denomination, denominated};
use osk_ui::widgets::keyboard::KeyInput;

const SIGNED: &[u8] = include_bytes!("../../../tools/vectors/signed-message.txt");
const DEMO_PSBT: &[u8] = include_bytes!("../../../tools/vectors/psbt/demo-regtest.psbt");

/// BIP-32 test vector 1: the master extended public key of seed
/// `000102030405060708090a0b0c0d0e0f`.
const VECTOR_1_XPUB: &str = "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8";

/// The same vector's master public key, compressed.
const VECTOR_1_PUBKEY: &str = "0339a36013301597daef41fbe593a02cc513d0b55527ec2df1050e2e8ff49c85c2";

/// BIP-32 test vector 1: the identifier of that key, which is HASH160
/// of the public key above. Its first four bytes are the fingerprint
/// `3442193e` the vector states.
const VECTOR_1_IDENTIFIER: &str = "3442193e1bb70916e914552172cd4e2dbc9df811";

/// Home › Tools.
fn open_tools(h: &mut Harness) {
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    assert_eq!(h.app.screen(), ScreenKind::Tools);
}

/// Hex of some bytes, for comparing against a published vector.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| alloc_hex(*b)).collect()
}

fn alloc_hex(b: u8) -> String {
    format!("{b:02x}")
}

#[test]
fn the_tools_that_are_built_open_and_come_back() {
    for (row, kind) in [
        (ids::TOOLS_EXPLORER, ScreenKind::Explore),
        (ids::TOOLS_WORD_LIST, ScreenKind::WordList),
        (ids::TOOLS_DICE, ScreenKind::DicePassphrase),
        (ids::MSG_CHECK_ROW, ScreenKind::Scan),
        (ids::TOOLS_DECODE, ScreenKind::Scan),
    ] {
        let mut h = Harness::new(PANEL);
        open_tools(&mut h);
        h.tap(row);
        assert_eq!(h.app.screen(), kind);
        h.tap(ids::BACK);
        assert_eq!(h.app.screen(), ScreenKind::Tools, "the chevron comes back");
    }
}

/// No tool needs a key, so every row is live on a device with nothing
/// loaded: each one opens and the chevron comes back.
#[test]
fn every_calculator_is_live_with_nothing_loaded() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    open_tools(&mut h);
    assert!(h.app.dimmed_rows().is_empty(), "no tool is dimmed");
    let texts = h.app.texts();
    for label in [
        s.tools_hashes,
        s.tools_encodings,
        s.tools_descriptor_checksum,
        s.tools_convert_key,
        s.tools_units,
        s.tools_decode,
    ] {
        assert!(texts.iter().any(|t| t == label), "{label} is not listed");
    }
    for (i, _) in Tool::ALL.iter().enumerate() {
        let mut h = Harness::new(PANEL);
        open_tools(&mut h);
        h.tap(ids::at(ids::TOOLS_CALC_BASE, i));
        // Every calculator that takes a string opens on the scanner,
        // whose "Type" row is the field; Units takes a number and opens
        // on the field itself.
        if Tool::ALL[i] == Tool::Units {
            assert_eq!(h.app.screen(), ScreenKind::Tool);
        } else {
            assert_eq!(h.app.screen(), ScreenKind::Scan);
            h.tap(ids::SCAN_TYPE);
            assert_eq!(h.app.screen(), ScreenKind::Tool);
        }
        h.tap(ids::BACK);
        assert_eq!(h.app.screen(), ScreenKind::Tools);
    }
}

/// Checking a signed message needs no key of this device's, so it is a
/// tool: the row opens the scanner, and what it reads is the answer.
#[test]
fn verify_a_signed_message_is_a_tool_and_reaches_the_answer() {
    let mut h = Harness::new(PANEL);
    open_tools(&mut h);
    assert!(
        h.app.texts().iter().any(|t| t == strings::EN.msg_check_row),
        "{:?}",
        h.app.texts()
    );
    h.tap(ids::MSG_CHECK_ROW);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: SIGNED.to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::CheckedMessage);
    assert!(
        h.app
            .checked_message()
            .expect("an answer")
            .checked()
            .is_some(),
        "the committed message is signed"
    );
}

/// FIPS 180-4 and BIP-32: the hashes a person compares against a
/// published value.
#[test]
fn the_hashes_match_the_published_vectors() {
    let abc = tools::hashes(b"abc");
    assert_eq!(
        hex(&abc.sha256),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let empty = tools::hashes(b"");
    assert_eq!(
        hex(&empty.sha256),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex(&empty.sha256d),
        "5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456"
    );
    let key = tools::read_input(VECTOR_1_PUBKEY, ReadAs::Hex);
    assert_eq!(hex(&tools::hashes(&key).hash160), VECTOR_1_IDENTIFIER);
}

/// The mode row decides what the field means: "dead" is four characters
/// of text or two bytes of hex, and they hash to different things.
#[test]
fn the_mode_row_decides_whether_the_field_is_text_or_hex() {
    let text = tools::read_input("dead", ReadAs::Text);
    let bytes = tools::read_input("dead", ReadAs::Hex);
    assert_eq!(text.len(), 4);
    assert_eq!(bytes, [0xde, 0xad]);
    // With no mode forced, a string that spells hex is read as hex.
    assert_eq!(tools::read_input("dead", ReadAs::Auto), bytes);
    assert_eq!(tools::read_input("dea", ReadAs::Auto).len(), 3);
}

/// BIP-173 and BIP-350: the strings the standards say are valid and the
/// ones they say must be rejected.
#[test]
fn the_bech32_vectors_are_read_the_way_the_standards_say() {
    use osk_codec::encodings::{Encoding, read};
    for valid in [
        "A12UEL5L",
        "a12uel5l",
        "abcdef1qpzry9x8gf2tvdw0s3jn54khce6mua7lmqqqxw",
        "?1ezyfcl",
    ] {
        assert_eq!(
            read(valid).map(|r| r.encoding),
            Some(Encoding::Bech32),
            "{valid}"
        );
    }
    for valid in [
        "A1LQFN3A",
        "abcdef1l7aum6echk45nj3s0wdvt2fg8x9yrzpqzd3ryx",
        "?1v759aa",
    ] {
        assert_eq!(
            read(valid).map(|r| r.encoding),
            Some(Encoding::Bech32m),
            "{valid}"
        );
    }
    for invalid in [
        // An empty human-readable part.
        "1pzry9x0s0muk",
        // A character outside the data charset.
        "x1b4n0q5v",
        // The checksum does not hold.
        "li1dgmt3",
        // Mixed case.
        "A1G7SGD8",
    ] {
        assert!(
            !matches!(
                read(invalid).map(|r| r.encoding),
                Some(Encoding::Bech32 | Encoding::Bech32m)
            ),
            "{invalid} was read as bech32"
        );
    }
}

/// BIP-380: the checksum of a descriptor, and the verdict on the one it
/// arrived with.
#[test]
fn the_descriptor_checksum_matches_bip_380() {
    let f = tools::checksum_facts("raw(deadbeef)").expect("a descriptor");
    assert_eq!(f.with_checksum, "raw(deadbeef)#89f8spxm");
    assert_eq!(f.given, None);

    let holds = tools::checksum_facts("raw(deadbeef)#89f8spxm").expect("a descriptor");
    assert_eq!(
        holds.given.as_ref().map(|(g, ok)| (g.as_str(), *ok)),
        Some(("89f8spxm", true))
    );

    // BIP-380's invalid examples: a checksum of the wrong length, and
    // one whose characters are not the ones this descriptor produces.
    for wrong in ["raw(deadbeef)#89f8spxmx", "raw(deadbeef)#89f8spxn"] {
        let f = tools::checksum_facts(wrong).expect("a descriptor");
        assert_eq!(f.with_checksum, "raw(deadbeef)#89f8spxm");
        assert!(!f.given.expect("a checksum arrived").1, "{wrong} held");
    }

    // Only a descriptor this device could derive from is one it offers
    // to load: a raw script is a descriptor and not a wallet.
    assert!(
        tools::checksum_facts("raw(deadbeef)")
            .expect("a descriptor")
            .wallet
            .is_none()
    );
    let wpkh = format!("wpkh([3442193e/84h/0h/0h]{VECTOR_1_XPUB}/<0;1>/*)");
    assert!(
        tools::checksum_facts(&wpkh)
            .expect("a descriptor")
            .wallet
            .is_some()
    );
}

/// The checksum screen states its verdict, the checksum it computed, and
/// the descriptor it computed it from. A descriptor this device can
/// derive from carries the row that loads it as a wallet; one it cannot
/// does not.
#[test]
fn the_checksum_screen_states_the_verdict_and_the_checksum_it_computed() {
    let s = &strings::EN;
    for (typed, verdict) in [
        ("raw(deadbeef)#89f8spxm", s.tool_checksum_valid),
        ("raw(deadbeef)#89f8spxn", s.tool_checksum_wrong),
        ("raw(deadbeef)", s.tool_checksum_missing),
    ] {
        let mut h = Harness::new(PANEL);
        open_tools(&mut h);
        h.tap(ids::at(ids::TOOLS_CALC_BASE, 2));
        h.tap(ids::SCAN_TYPE);
        h.type_text(typed);
        h.key(osk_shell_api::Key::Enter);
        let texts = h.app.texts();
        assert!(
            texts.iter().any(|t| t == verdict),
            "{typed}: {verdict} in {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t == "#89f8spxm"),
            "{typed}: the computed checksum in {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t == s.tool_computed_checksum_row),
            "{typed}: the row it sits on in {texts:?}"
        );
        assert!(
            !texts.iter().any(|t| t == s.tool_policy_load),
            "{typed}: a raw script is no wallet to load"
        );
        assert!(
            h.app.rect_of(ids::TOOL_LOAD_WALLET).is_none(),
            "{typed}: and there is no row for it"
        );
    }

    // A single-key descriptor this device can derive from: the row is
    // there, and it opens the wallet review.
    let wpkh = format!("wpkh([3442193e/84h/0h/0h]{VECTOR_1_XPUB}/<0;1>/*)");
    let mut h = Harness::new(PANEL);
    open_tools(&mut h);
    h.tap(ids::at(ids::TOOLS_CALC_BASE, 2));
    h.tap(ids::SCAN_TYPE);
    h.type_text(&wpkh);
    h.key(osk_shell_api::Key::Enter);
    assert!(
        h.app.texts().iter().any(|t| t == s.tool_policy_load),
        "{:?}",
        h.app.texts()
    );
    h.tap(ids::TOOL_LOAD_WALLET);
    assert_ne!(h.app.screen(), ScreenKind::ToolResult, "the review opened");
}

/// SLIP-132: one key, five spellings, and nothing invented. The key
/// material is the same in each, so every spelling decodes back to the
/// key that was typed.
#[test]
fn one_key_is_shown_in_every_spelling() {
    let KeyReading::Public(f) = tools::key_facts(VECTOR_1_XPUB, Network::Mainnet) else {
        panic!("the BIP-32 vector 1 master key is an extended public key");
    };
    assert_eq!(f.bip32, VECTOR_1_XPUB);
    assert_eq!(f.depth, 0);
    assert_eq!(f.fingerprint, "3442193e");
    assert_eq!(f.child, "0");
    assert_eq!(f.network, Network::Mainnet);
    let prefixes = ["xpub", "ypub", "zpub", "xpub"];
    for ((_, spelling), prefix) in f.slip132.iter().zip(prefixes) {
        assert!(spelling.starts_with(prefix), "{spelling} is not a {prefix}");
        let (decoded, _) = slip132::decode_xpub(spelling).expect("a key");
        assert_eq!(
            hex(&decoded.public_key.serialize()),
            VECTOR_1_PUBKEY,
            "{spelling} is another key"
        );
    }

    // A private key is refused: the key explorer is where those are
    // seen.
    let xprv = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";
    assert_eq!(
        tools::key_facts(xprv, Network::Mainnet),
        KeyReading::Private
    );
    assert_eq!(
        tools::key_facts("nonsense", Network::Mainnet),
        KeyReading::None
    );
}

/// The units at the boundaries: the smallest amount there is, and the
/// largest there will ever be.
#[test]
fn the_units_convert_at_the_boundaries() {
    assert_eq!(denominated(1, Denomination::Sat), "1");
    assert_eq!(denominated(1, Denomination::Btc), "0.00000001");
    assert_eq!(denominated(1, Denomination::MBtc), "0.00001");
    assert_eq!(denominated(1, Denomination::Bits), "0.01");

    let all = 21_000_000 * Denomination::Btc.sat();
    assert_eq!(all, tools::MAX_SATS);
    assert_eq!(
        denominated(all, Denomination::Sat),
        "2\u{00A0}100\u{00A0}000\u{00A0}000\u{00A0}000\u{00A0}000"
    );
    assert_eq!(
        denominated(all, Denomination::Btc),
        "21\u{00A0}000\u{00A0}000.00000000"
    );
    assert_eq!(
        denominated(all, Denomination::Bits),
        "21\u{00A0}000\u{00A0}000\u{00A0}000\u{00A0}000.00"
    );
}

/// Typing an amount and changing the unit it is typed in. A phone has
/// the room to be the answer: the other three units are on the screen
/// as the digits go in. A 240 dp panel does not, so ✓ opens the same
/// three as a Record rather than showing half a row.
#[test]
fn the_units_screen_is_the_answer_where_there_is_room_for_it() {
    let s = &strings::EN;
    let mut h = Harness::new(PHONE);
    open_tools(&mut h);
    h.tap(ids::at(ids::TOOLS_CALC_BASE, 4));
    assert_eq!(h.app.screen(), ScreenKind::Tool);
    h.pad(ids::TOOL_KEYBOARD, KeyInput::Char('1'));
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == "0.00000001"),
        "one satoshi in BTC: {texts:?}"
    );
    // The mode row opens the Choice that says which unit is typed.
    h.tap(ids::TOOL_MODE);
    h.tap(ids::at(ids::PICK_BASE, 1));
    h.tap(ids::PICK_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Tool);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.tool_btc),
        "the mode row names BTC: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "100\u{00A0}000\u{00A0}000"),
        "one bitcoin in satoshi: {texts:?}"
    );
}

/// On a 240 dp panel the field carries the mode row alone and ✓ opens
/// the Record with the other three units.
#[test]
fn a_small_panel_reads_the_units_on_a_record() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    open_tools(&mut h);
    h.tap(ids::at(ids::TOOLS_CALC_BASE, 4));
    h.pad(ids::TOOL_KEYBOARD, KeyInput::Char('1'));
    let texts = h.app.texts();
    assert!(
        !texts.iter().any(|t| t == "0.00000001"),
        "the panel shows the mode row and the field: {texts:?}"
    );
    h.pad(ids::TOOL_KEYBOARD, KeyInput::Done);
    assert_eq!(h.app.screen(), ScreenKind::ToolResult);
    let texts = h.app.texts();
    for (label, value) in [
        (s.tool_btc, "0.00000001"),
        (s.tool_mbtc, "0.00001"),
        (s.tool_bits, "0.01"),
    ] {
        assert!(texts.iter().any(|t| t == label), "{label}: {texts:?}");
        assert!(texts.iter().any(|t| t == value), "{value}: {texts:?}");
    }
}

/// Typing into a calculator and pressing ✓ reaches its answer, and the
/// chevron comes back to the field with what was typed still in it.
#[test]
fn a_calculator_reaches_its_answer_and_comes_back() {
    let mut h = Harness::new(PANEL);
    open_tools(&mut h);
    h.tap(ids::at(ids::TOOLS_CALC_BASE, 0));
    h.tap(ids::SCAN_TYPE);
    h.type_text("abc");
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::ToolResult);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t.contains("ba78") || t.contains("15ad")),
        "the SHA-256 of abc: {:?}",
        h.app.texts()
    );
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Tool);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Tools);
}

/// Decode a transaction: a PSBT read from a file with no key loaded
/// walks the Sign review's own screens, reaches the outputs, and ends
/// with Done rather than a confirm.
#[test]
fn decode_a_transaction_reaches_the_outputs_and_ends_with_done() {
    let mut h = Harness::new(PANEL);
    open_tools(&mut h);
    h.tap(ids::TOOLS_DECODE);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO_PSBT.to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Decode);
    // The first page is the summary, and its way on is live even though
    // no loaded key signs anything here.
    let mut saw_outputs = false;
    for _ in 0..12 {
        if h.app.rect_of(ids::DECODE_DONE).is_some() {
            break;
        }
        if h.app.rect_of(ids::at(ids::SIGN_OUT_BASE, 0)).is_some() {
            saw_outputs = true;
        }
        assert!(
            h.app.rect_of(ids::SIGN_CONTINUE).is_some(),
            "a way on: {:?}",
            h.app.texts()
        );
        h.tap(ids::SIGN_CONTINUE);
    }
    assert!(saw_outputs, "the outputs page");
    assert!(
        h.app.rect_of(ids::DECODE_DONE).is_some(),
        "the last page is Done: {:?}",
        h.app.texts()
    );
    assert!(
        h.app.rect_of(ids::SIGN_HOLD).is_none(),
        "nothing is signed here"
    );
    h.tap(ids::DECODE_DONE);
    assert_eq!(h.app.screen(), ScreenKind::Home);
}

/// A transaction being read is read with whatever this device holds:
/// change of a wallet in use is verified, and with nothing loaded the
/// change row says there was nothing to check it against.
#[test]
fn a_decoded_transaction_recognises_a_wallet_in_use() {
    const POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.policy");
    const PSBT: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.psbt");
    let s = &strings::EN;
    let psbt = Psbt::parse_base64(PSBT.trim())
        .expect("the committed PSBT")
        .to_bytes();

    let decode = |h: &mut Harness| {
        h.go_home();
        open_tools(h);
        h.tap(ids::TOOLS_DECODE);
        h.send(Event::CameraUnavailable);
        h.tap(ids::SCAN_FILE);
        h.send(Event::File {
            kind: FileKind::Any,
            bytes: psbt.clone(),
        });
        assert_eq!(h.app.screen(), ScreenKind::Decode);
    };

    // The wallet in use: its change is change, and the review says so
    // the way the Sign review does.
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(3);
    h.tap(ids::HOME_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: POLICY.as_bytes().to_vec(),
    });
    h.tap(ids::INSPECT_USE_WALLET);
    decode(&mut h);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t.eq_ignore_ascii_case(s.row_mine)),
        "the change is this wallet's: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t == s.sign_change_unverified),
        "no verdict about unverified change: {texts:?}"
    );

    // Nothing loaded: there was nothing to recognise change against,
    // and the row states that rather than a danger.
    let mut h = Harness::new(PANEL);
    h.set_network(3);
    decode(&mut h);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.decode_no_key),
        "the change row says nothing was loaded: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t == s.sign_change_unverified),
        "no verdict about unverified change: {texts:?}"
    );
}

/// A raw transaction in hex reads the same way: it is wrapped in a PSBT
/// so that the same inspection and the same screens serve both.
#[test]
fn a_raw_transaction_in_hex_is_decoded_too() {
    // The genesis coinbase transaction, which every implementation
    // agrees on.
    const GENESIS: &str = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000";
    let (wrapper, txid) = tools::raw_transaction(GENESIS.as_bytes()).expect("a transaction");
    assert_eq!(
        format!("{txid}"),
        "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b"
    );
    let mut h = Harness::new(PANEL);
    open_tools(&mut h);
    h.tap(ids::TOOLS_DECODE);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: GENESIS.as_bytes().to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Decode);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t.contains("4a5e") || t.contains("a33b")),
        "the transaction id: {:?}",
        h.app.texts()
    );
    assert!(!wrapper.is_empty());
}

/// The mode row above a calculator's field is read, not guessed at: on
/// the short panels, where the five-row passphrase keyboard leaves it
/// the least room of anywhere, it is still drawn whole above the field
/// rather than under it.
#[test]
fn a_calculators_mode_row_is_drawn_above_its_field_on_the_short_panels() {
    for display in [PANEL, TINY] {
        for (tool, index) in [(Tool::Hashes, 0), (Tool::Miniscript, 5)] {
            let mut h = Harness::new(display);
            open_tools(&mut h);
            h.tap(ids::at(ids::TOOLS_CALC_BASE, index));
            h.tap(ids::SCAN_TYPE);
            assert_eq!(h.app.screen(), ScreenKind::Tool);
            let mode = h
                .app
                .rect_of(ids::TOOL_MODE)
                .unwrap_or_else(|| panic!("{tool:?} has a mode row"));
            let field = h
                .app
                .rect_of(osk_ui::screens::FIELD)
                .expect("a calculator types into a field");
            assert!(
                mode.y + mode.h <= field.y,
                "{tool:?} on {}x{}: the mode row {mode:?} runs into the field {field:?}",
                display.width,
                display.height,
            );
            assert!(mode.h > 0, "{tool:?}: the mode row has a height");
        }
    }
}

/// Tools › Compare transactions (`docs/PLANNING.md` §16.111 rule 4):
/// the same transaction read twice is one transaction, and two
/// different ones are listed by what differs.
#[test]
fn two_transactions_are_read_one_after_the_other_and_compared() {
    const PSBT: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.psbt");
    const OTHER: &str = include_str!("../../../tools/vectors/psbt/wallet-liana.psbt");
    let s = &strings::EN;
    let bytes = |text: &str| {
        Psbt::parse_base64(text.trim())
            .expect("a committed fixture")
            .to_bytes()
    };

    let read = |h: &mut Harness, id: osk_ui::Id, psbt: Vec<u8>| {
        h.tap(id);
        h.send(Event::CameraUnavailable);
        h.tap(ids::SCAN_FILE);
        h.send(Event::File {
            kind: FileKind::Any,
            bytes: psbt,
        });
    };

    let mut h = Harness::new(PANEL);
    open_tools(&mut h);
    read(&mut h, ids::TOOLS_COMPARE, bytes(PSBT));
    assert_eq!(h.app.screen(), ScreenKind::CompareTx);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.compare_tx_read_second),
        "the tool asks for the second transaction: {texts:?}"
    );
    read(&mut h, ids::COMPARE_TX_SECOND, bytes(PSBT));
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.compare_tx_same),
        "the same bytes twice are one transaction: {texts:?}"
    );

    // Another wallet's transaction: the fields that differ are rows.
    h.tap(ids::COMPARE_TX_DONE);
    open_tools(&mut h);
    read(&mut h, ids::TOOLS_COMPARE, bytes(PSBT));
    read(&mut h, ids::COMPARE_TX_SECOND, bytes(OTHER));
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.compare_tx_different),
        "two transactions are not one: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains(s.cmp_spends)),
        "the outpoint each one spends is one of the rows: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains(s.cmp_witness_script)),
        "and so is the script being spent: {texts:?}"
    );
    h.tap(ids::COMPARE_TX_DONE);
    assert_eq!(h.app.screen(), ScreenKind::Home);
}

// ----- Notes (`docs/PLANNING.md` §16.112 rule 2) -----

/// Tools › Notes › New note: what is typed is shown whole, sealed and
/// shown as one QR while it is short, exported as a file, and forgotten.
#[test]
fn a_note_is_typed_shown_exported_and_forgotten() {
    let note = "Keys in the safe. Words with the notary.";
    let mut h = Harness::new(PANEL);
    open_tools(&mut h);
    h.tap(ids::TOOLS_NOTES);
    assert_eq!(h.app.screen(), ScreenKind::Notes);
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.notes_none)),
        "a device that has made no note lists none: {:?}",
        h.app.texts()
    );

    h.tap(ids::NOTES_NEW);
    assert_eq!(h.app.screen(), ScreenKind::NoteText);
    h.type_text(note);
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Note);
    assert!(
        h.app.texts().iter().any(|t| t.contains(note)),
        "the note is shown whole: {:?}",
        h.app.texts()
    );

    // A short note's sealed file fits one QR, so its Result offers the
    // code.
    h.tap(ids::NOTE_EXPORT);
    assert_eq!(h.app.screen(), ScreenKind::ExportForm);
    h.tap(ids::FORM_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::SealPass);
    h.type_text("a long enough one");
    h.key(osk_shell_api::Key::Enter);
    h.type_text("a long enough one");
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Sealed);
    h.tap(ids::SEAL_SHOW_QR);
    assert_eq!(h.app.screen(), ScreenKind::SealedQr);
    assert_eq!(h.app.qr_visible(), Some(true));

    h.go_home();
    open_tools(&mut h);
    h.tap(ids::TOOLS_NOTES);
    h.tap(ids::at(ids::FORM_BASE, 0));
    assert_eq!(
        h.app.screen(),
        ScreenKind::Note,
        "the note is still in hand"
    );

    // Saved as a file, the shell is handed an `osk-backup`.
    h.tap(ids::NOTE_EXPORT);
    h.tap(ids::at(ids::FORM_BASE, 0));
    h.tap(ids::FORM_CONTINUE);
    h.type_text("a long enough one");
    h.key(osk_shell_api::Key::Enter);
    h.type_text("a long enough one");
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Sealed);
    h.seen.clear();
    h.tap(ids::SEAL_SAVE);
    let bytes = h
        .seen
        .iter()
        .find_map(|c| match c {
            osk_shell_api::Command::WriteFile { bytes, .. } => Some(bytes.clone()),
            _ => None,
        })
        .expect("the note was written");
    match osk_backup::oskb::open_payload(&bytes, b"a long enough one")
        .expect("the passphrase opens it")
    {
        osk_backup::oskb::Opened::Note(text) => {
            assert_eq!(String::from_utf8(text.to_vec()).expect("text"), note);
        }
        _ => panic!("a note"),
    }

    // The shell says the file is there, and the Result says so.
    h.send(Event::FileWritten {
        kind: FileKind::Any,
    });
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t.contains(strings::EN.seal_file_name)),
        "the row says the saved name: {:?}",
        h.app.texts()
    );

    // Forgetting it leaves the Menu with nothing listed.
    h.tap(ids::SEAL_DONE);
    open_tools(&mut h);
    h.tap(ids::TOOLS_NOTES);
    h.tap(ids::at(ids::FORM_BASE, 0));
    h.tap(ids::NOTE_FORGET);
    assert_eq!(h.app.screen(), ScreenKind::Notes);
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.notes_none)),
        "the note was forgotten: {:?}",
        h.app.texts()
    );
}

/// A note whose sealed file is too long for one code is not offered as
/// one: its Result has no "Show as QR".
#[test]
fn a_long_note_is_not_offered_as_a_qr() {
    let mut h = Harness::new(PANEL);
    open_tools(&mut h);
    h.tap(ids::TOOLS_NOTES);
    h.tap(ids::NOTES_NEW);
    for _ in 0..40 {
        h.type_text("the third key is with the notary. ");
    }
    h.key(osk_shell_api::Key::Enter);
    h.tap(ids::NOTE_EXPORT);
    assert_eq!(h.app.screen(), ScreenKind::ExportForm);
    h.tap(ids::FORM_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::SealPass);
    h.type_text("a long enough one");
    h.key(osk_shell_api::Key::Enter);
    h.type_text("a long enough one");
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Sealed);
    assert!(
        h.app.rect_of(ids::SEAL_SAVE).is_some(),
        "the file is still offered"
    );
    assert!(
        h.app.rect_of(ids::SEAL_SHOW_QR).is_none(),
        "no code to show: {:?}",
        h.app.texts()
    );
}

/// A note read from a plain text file lands on the same Document, and a
/// file that is not text is refused with a reason.
#[test]
fn a_note_is_read_from_a_plain_file() {
    let mut h = Harness::new(PANEL);
    open_tools(&mut h);
    h.tap(ids::TOOLS_NOTES);
    h.tap(ids::NOTES_READ_FILE);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: b"Keys in the safe.".to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Note);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t.contains("Keys in the safe.")),
        "the file's text: {:?}",
        h.app.texts()
    );
}
