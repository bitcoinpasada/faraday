//! The Sign flow through the shell contract (`docs/UX.md` §7.3): the file
//! channel round trip, every wizard step, the risk acknowledgement, and
//! the output compared with `osk-psbt` used directly.

mod common;

use common::{ABANDON, Harness, PANEL, PHONE, TINY};
use opensigner_core::ids;
use opensigner_core::scan::ScanStage;
use opensigner_core::sign::{Stage, Step};
use opensigner_core::{ScreenKind, strings};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network};
use osk_psbt::bitcoin::consensus::encode::serialize_hex;
use osk_psbt::{Aux, Context, KeyRef, Nonce, Psbt, WarningKind};
use osk_shell_api::{
    App, BootState, Command, DisplayInfo, Event, FileKind, SecureHardware, TouchPhase,
};
use osk_ui::components::OutputBadge;
use osk_ui::widgets::Icon;

const DEMO: &[u8] = include_bytes!("../../../tools/vectors/psbt/demo-regtest.psbt");
/// A spoofed change output: the origin claim stays, the script pays the
/// recipient, so it carries a `ChangeSpoof` danger and, because two
/// outputs now share an address, an `AddressReuse` caution.
const SPOOF: &[u8] = include_bytes!("../../../tools/vectors/psbt/warn-change-spoof.psbt");
/// A fee of 25 % of the amount sent: one danger, nothing else.
const HIGH_FEE: &[u8] = include_bytes!("../../../tools/vectors/psbt/warn-high-fee.psbt");
/// A 200 sat output: one caution, nothing else.
const DUST: &[u8] = include_bytes!("../../../tools/vectors/psbt/warn-dust-output.psbt");
/// A locktime of block 800 000: one piece of information, nothing else.
const LOCKTIME: &[u8] = include_bytes!("../../../tools/vectors/psbt/warn-locktime-in-future.psbt");
/// Change the device cannot derive from any loaded key: one danger.
const UNVERIFIED: &[u8] = include_bytes!("../../../tools/vectors/psbt/warn-unverified-change.psbt");
/// Two p2wpkh inputs, neither carrying the transaction it spends: one
/// block.
const AMOUNT_UNVERIFIED: &[u8] =
    include_bytes!("../../../tools/vectors/psbt/warn-amount-unverified.psbt");

fn master() -> MasterKey {
    let m = Mnemonic::parse(Language::English, &ABANDON.join(" ")).unwrap();
    MasterKey::from_seed(&m.to_seed(b"").unwrap(), Network::Regtest)
}

fn demo_psbt() -> Psbt {
    Psbt::parse_base64(std::str::from_utf8(DEMO).unwrap()).unwrap()
}

impl Harness {
    /// Taps `id` without draining, and returns the commands it produced.
    fn tap_commands(&mut self, id: osk_ui::Id) -> Vec<Command> {
        let (x, y) = self.center(id);
        self.app.event(Event::Touch {
            x,
            y,
            phase: TouchPhase::Down,
        });
        self.app.event(Event::Touch {
            x,
            y,
            phase: TouchPhase::Up,
        });
        self.drain()
    }

    /// "Read a file" under the scanner's viewfinder, asserting the
    /// request goes out.
    fn request_file(&mut self) {
        let commands = self.tap_commands(ids::SCAN_FILE);
        assert!(
            commands.contains(&Command::RequestFile {
                kind: FileKind::Any
            }),
            "{commands:?}"
        );
        assert_eq!(self.app.scan_stage(), Some(ScanStage::Waiting));
    }

    /// The wallet of the first key → "Sign a transaction" → "Read a
    /// file". A transaction reaches the flow by arriving, and the
    /// scanner is where it arrives.
    fn open_sign_and_request(&mut self) {
        self.open_single_sig(0);
        self.tap(ids::WALLET_SIGN);
        assert_eq!(self.app.screen(), ScreenKind::Scan);
        self.request_file();
    }

    /// The test key, so that a wallet page exists to sign from.
    fn with_key(&mut self) {
        self.start_load(&ABANDON);
        self.finish_load(None);
    }

    /// Loads the test key, switches to regtest, opens Sign and answers the
    /// file request with `bytes`.
    fn sign_with(&mut self, bytes: &[u8]) {
        self.start_load(&ABANDON);
        self.finish_load(None);
        self.set_network(3);
        assert_eq!(self.app.network(), Network::Regtest);
        self.open_sign_and_request();
        self.send(Event::File {
            kind: FileKind::Any,
            bytes: bytes.to_vec(),
        });
    }

    /// Taps Continue through the review steps up to Confirm. Continue
    /// walks the outputs one at a time and then leaves the run.
    fn advance_to_confirm(&mut self) {
        assert_eq!(self.app.sign_stage(), Some(Stage::Wizard(Step::Summary)));
        self.tap(ids::SIGN_CONTINUE);
        assert_eq!(self.app.sign_stage(), Some(Stage::Wizard(Step::Outputs)));
        assert!(self.app.rect_of(ids::at(ids::SIGN_OUT_BASE, 0)).is_some());
        assert!(
            self.app.rect_of(ids::at(ids::SIGN_OUT_BASE, 1)).is_none(),
            "one output per screen"
        );
        self.tap(ids::SIGN_CONTINUE);
        assert!(self.app.rect_of(ids::at(ids::SIGN_OUT_BASE, 1)).is_some());
        self.tap(ids::SIGN_CONTINUE);
        assert_eq!(self.app.sign_stage(), Some(Stage::Wizard(Step::Inputs)));
        self.tap(ids::SIGN_CONTINUE);
        // Warnings get their own step only when there are any.
        if self.app.sign_stage() == Some(Stage::Wizard(Step::Warnings)) {
            if self.app.rect_of(ids::SIGN_ACK).is_some() {
                self.tap(ids::SIGN_ACK);
            }
            self.tap(ids::SIGN_CONTINUE);
        }
        assert_eq!(self.app.sign_stage(), Some(Stage::Wizard(Step::Confirm)));
    }

    /// Continue until the warnings step, which follows the summary, the
    /// outputs one at a time, and the inputs.
    fn advance_to_warnings(&mut self) {
        for _ in 0..8 {
            if self.app.sign_stage() == Some(Stage::Wizard(Step::Warnings)) {
                return;
            }
            self.tap(ids::SIGN_CONTINUE);
        }
        panic!(
            "the warnings step was never reached: {:?}",
            self.app.sign_stage()
        );
    }

    /// The one `WriteFile` that "Save to file" produces.
    fn save(&mut self) -> (FileKind, String, Vec<u8>) {
        let commands = self.tap_commands(ids::SIGN_SAVE);
        let mut written = commands.into_iter().filter_map(|c| match c {
            Command::WriteFile {
                kind,
                name_hint,
                bytes,
            } => Some((kind, name_hint, bytes)),
            _ => None,
        });
        let first = written.next().expect("one WriteFile");
        assert!(written.next().is_none(), "exactly one WriteFile");
        first
    }
}

/// With nothing loaded there is no wallet to sign from: Wallets is its
/// own empty state, and none of its rows signs.
#[test]
fn there_is_no_way_to_sign_with_no_key_loaded() {
    let mut h = Harness::new(PANEL);
    h.open_wallets();
    for id in [ids::WALLET_SIGN, ids::WALLET_SIGN_MESSAGE] {
        assert!(h.app.rect_of(id).is_none(), "row {} is live", id.0);
    }
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_none(),
        "no wallet is listed"
    );
}

/// A transaction still reaches the review with no key, through the
/// scanner's file channel, and there is no way on from it: the review's
/// action is dead and the danger names the missing key.
#[test]
fn a_psbt_can_be_inspected_without_a_key_but_not_signed() {
    let mut h = Harness::new(PANEL);
    h.tap(ids::HOME_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Summary)));
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(insp.participating_keys.is_empty());
    assert!(
        insp.dangers().contains(&WarningKind::NoParticipatingKey),
        "{:?}",
        insp.warnings
    );
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(
        h.app.sign_stage(),
        Some(Stage::Wizard(Step::Summary)),
        "the review has no way on with no key"
    );
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(h.app.sign_inspection().is_none(), "PSBT dropped");
}

/// §4.11: the dead Continue carries the condition the person could
/// change. Reading a transaction before loading a key is an ordinary
/// thing to do, and what to do about it is not otherwise on the screen.
#[test]
fn a_review_that_cannot_go_on_says_what_to_change() {
    let s = &strings::EN;
    let read_demo = |h: &mut Harness| {
        h.tap(ids::HOME_SCAN);
        h.send(Event::CameraUnavailable);
        h.tap(ids::SCAN_FILE);
        h.send(Event::File {
            kind: FileKind::Any,
            bytes: DEMO.to_vec(),
        });
        assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Summary)));
    };

    let mut h = Harness::new(PANEL);
    read_demo(&mut h);
    assert!(
        h.app.texts().iter().any(|t| t == s.sign_no_key),
        "{:?}",
        h.app.texts()
    );

    // A key is loaded, and it signs nothing in this transaction.
    h.tap(ids::BACK);
    h.start_load(&ABANDON);
    h.finish_load(Some("another wallet"));
    read_demo(&mut h);
    let texts = h.app.texts();
    assert!(texts.iter().any(|t| t == s.sign_keys_not_in), "{texts:?}");
    assert!(!texts.iter().any(|t| t == s.sign_no_key), "{texts:?}");

    // The key the transaction is for: the way on is live and says
    // nothing.
    let mut g = Harness::new(PANEL);
    g.with_key();
    g.set_network(3);
    read_demo(&mut g);
    let texts = g.app.texts();
    assert!(!texts.iter().any(|t| t == s.sign_no_key), "{texts:?}");
    assert!(!texts.iter().any(|t| t == s.sign_keys_not_in), "{texts:?}");
    g.tap(ids::SIGN_CONTINUE);
    assert_eq!(g.app.sign_stage(), Some(Stage::Wizard(Step::Outputs)));
}

#[test]
fn the_full_path_writes_the_final_transaction_osk_psbt_produces() {
    let mut h = Harness::new(PANEL);
    h.sign_with(DEMO);
    let insp = h.app.sign_inspection().expect("inspected");
    assert_eq!(insp.participating_keys.len(), 1);
    assert!(!insp.has_danger(), "{:?}", insp.warnings);
    assert!(h.app.rect_of(ids::SIGN_BACK).is_none());
    h.advance_to_confirm();
    assert!(h.app.sign_enabled());
    assert!(
        h.app.rect_of(ids::SIGN_ACK).is_none(),
        "no danger, no acknowledgement"
    );
    h.hold(ids::SIGN_HOLD);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Result)));

    let (kind, name_hint, bytes) = h.save();
    assert_eq!(kind, FileKind::Any);
    assert_eq!(name_hint, "signed.txn");
    // The file exists once the shell says so, and the result offers Done.
    h.send(Event::FileWritten { kind });

    // The same thing, straight through osk-psbt.
    let master = master();
    let keys = [KeyRef::from_master(&master, 0).unwrap()];
    let ctx = Context {
        network: Network::Regtest,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let mut psbt = demo_psbt();
    let r = osk_psbt::sign(
        &mut psbt,
        &[&master],
        &[master.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
    )
    .unwrap();
    assert!(r.complete);
    let tx = osk_psbt::finalize(&mut psbt).unwrap().expect("complete");
    assert_eq!(String::from_utf8(bytes).unwrap(), serialize_hex(&tx));

    // The Signatures menu lists the transaction id and the signature,
    // each a reference row that opens the whole string on Compare.
    h.tap(ids::SIGN_SIGNATURES);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Signatures)));
    assert!(h.app.rect_of(ids::SIGN_TXID).is_some(), "finalized");
    assert!(h.app.rect_of(ids::at(ids::SIGN_SIG_BASE, 0)).is_some());
    h.tap(ids::at(ids::SIGN_SIG_BASE, 0));
    assert!(
        h.app.rect_of(ids::COMPARE_TEXT).is_some(),
        "the whole bytes"
    );
    h.tap(ids::COMPARE_DONE);
    h.tap(ids::BACK);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Result)));
    h.tap(ids::SIGN_DONE);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(h.app.sign_inspection().is_none(), "PSBT dropped on exit");
    assert_eq!(h.app.fingerprints().len(), 1, "key stays loaded");
}

/// A change output that claims one of our keys and pays somewhere else is
/// evidence of a hostile coordinator, so the flow ends at the warning:
/// there is no acknowledgement to give and no way to the hold.
#[test]
fn a_spoofed_change_output_cannot_be_signed_at_all() {
    let mut h = Harness::new(PHONE);
    h.sign_with(SPOOF);
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(
        insp.blocked().contains(&WarningKind::ChangeSpoof),
        "{:?}",
        insp.warnings
    );
    h.tap(ids::SIGN_CONTINUE);
    h.tap(ids::SIGN_CONTINUE);
    h.tap(ids::SIGN_CONTINUE);
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Warnings)));
    assert!(!h.app.sign_enabled(), "the block holds the hold shut");
    assert!(
        h.app.rect_of(ids::SIGN_ACK).is_none(),
        "a block offers no acknowledgement"
    );
    // Continue does not leave the warnings either.
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(
        h.app.sign_stage(),
        Some(Stage::Wizard(Step::Warnings)),
        "the review ends at the block"
    );
    assert!(!h.app.sign_enabled());
}

/// §4.11: a caution is read and passed. The card names what is wrong in
/// the words the warning maps to, and the flow goes on to the hold.
#[test]
fn a_caution_is_shown_and_the_flow_goes_on_to_the_hold() {
    let mut h = Harness::new(PANEL);
    h.sign_with(DUST);
    h.advance_to_warnings();
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == strings::EN.warn_output)
            && texts.iter().any(|t| t == strings::EN.warn_dust),
        "the dust card: {texts:?}"
    );
    assert!(
        h.app.rect_of(ids::SIGN_ACK).is_none(),
        "a caution asks for no acknowledgement"
    );
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Confirm)));
    assert!(h.app.sign_enabled());
    h.hold(ids::SIGN_HOLD);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Result)));
}

/// §4.11 Warning: "Nothing is cut and nothing is squeezed into half the
/// panel." On both panels the card names the field the warning is about
/// and says what is wrong with it, every word of both is on the screen,
/// and the value sits on one line under its label.
#[test]
fn a_warning_card_shows_its_label_and_its_value_whole() {
    for display in [PANEL, TINY] {
        for (bytes, label, value) in [
            (
                SPOOF,
                strings::EN.warn_blocked,
                strings::EN.warn_change_spoof,
            ),
            (DUST, strings::EN.warn_output, strings::EN.warn_dust),
        ] {
            let mut h = Harness::new(display);
            h.sign_with(bytes);
            h.advance_to_warnings();
            let texts = h.app.texts();
            let cut = h.app.cut_texts();
            let wrapped = h.app.wrapped_texts();
            assert!(
                !wrapped.iter().any(|t| t == value),
                "{value:?} runs onto a second line on the warnings screen at {}x{}",
                display.width,
                display.height
            );
            for word in [label, value] {
                assert!(
                    texts.iter().any(|t| t == word),
                    "{word:?} is not on the warnings screen at {}x{}: {texts:?}",
                    display.width,
                    display.height
                );
                assert!(
                    !cut.iter().any(|t| t == word),
                    "{word:?} is cut on the warnings screen at {}x{}: {cut:?}",
                    display.width,
                    display.height
                );
            }
        }
    }
}

/// A danger names itself and stops the flow until it is acknowledged;
/// after the acknowledgement the transaction signs.
#[test]
fn a_danger_stops_the_flow_until_it_is_acknowledged() {
    let mut h = Harness::new(PANEL);
    h.sign_with(HIGH_FEE);
    let insp = h.app.sign_inspection().expect("inspected");
    assert_eq!(
        insp.dangers(),
        vec![WarningKind::HighFee { pct_of_amount: 25 }],
        "{:?}",
        insp.warnings
    );
    h.advance_to_warnings();
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == strings::EN.sign_fee)
            && texts
                .iter()
                .any(|t| *t == strings::fill1(strings::EN.warn_high_fee, "25")),
        "the fee card: {texts:?}"
    );
    assert!(!h.app.sign_enabled(), "the danger blocks the hold");
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(
        h.app.sign_stage(),
        Some(Stage::Wizard(Step::Warnings)),
        "no way on without the acknowledgement"
    );
    h.tap(ids::SIGN_ACK);
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Confirm)));
    h.hold(ids::SIGN_HOLD);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Result)));
}

/// Two SegWit v0 inputs whose previous transactions are missing state
/// amounts that nothing proves, and two signing rounds over two
/// different lies pay the difference to the miner. The flow shows the
/// block and ends there, with no acknowledgement and no hold.
#[test]
fn a_segwit_input_without_its_previous_transaction_cannot_be_signed() {
    let mut h = Harness::new(PANEL);
    h.sign_with(AMOUNT_UNVERIFIED);
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(
        insp.blocked().contains(&WarningKind::AmountUnverified),
        "{:?}",
        insp.warnings
    );
    h.advance_to_warnings();
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == strings::EN.warn_blocked)
            && texts
                .iter()
                .any(|t| t == strings::EN.warn_amount_unverified),
        "the blocked card: {texts:?}"
    );
    assert!(
        h.app.rect_of(ids::SIGN_ACK).is_none(),
        "a block offers no acknowledgement"
    );
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(
        h.app.sign_stage(),
        Some(Stage::Wizard(Step::Warnings)),
        "the review ends at the block"
    );
    assert!(!h.app.sign_enabled());
}

/// One input signs only its own stated amount, so a lie about it gives a
/// signature no node accepts and there is no second input to combine it
/// with. A single-input transaction carrying only its `witness_utxo`
/// goes through the review to the hold and the signature.
#[test]
fn a_single_input_transaction_is_signed_without_its_previous_transaction() {
    let mut psbt = demo_psbt();
    assert_eq!(psbt.unsigned_tx().input.len(), 1);
    psbt.inner_mut().inputs[0].non_witness_utxo = None;
    let mut h = Harness::new(PANEL);
    h.sign_with(psbt.to_base64().as_bytes());
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(
        !insp.blocked().contains(&WarningKind::AmountUnverified),
        "{:?}",
        insp.warnings
    );
    h.advance_to_confirm();
    assert!(h.app.sign_enabled());
    h.hold(ids::SIGN_HOLD);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Result)));
    let (_, _, bytes) = h.save();
    assert!(!bytes.is_empty(), "the signed transaction");
}

/// Information is worth a card too: a locktime nobody needs to act on is
/// still on the screen before the hold.
#[test]
fn an_information_warning_is_still_shown() {
    let mut h = Harness::new(PANEL);
    h.sign_with(LOCKTIME);
    h.advance_to_warnings();
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == strings::EN.sign_locktime)
            && texts.iter().any(|t| t == strings::EN.warn_locktime),
        "the locktime card: {texts:?}"
    );
    assert!(h.app.rect_of(ids::SIGN_ACK).is_none());
}

/// The worst thing on the transaction is read first: the spoofed change
/// blocks signing, the address it reuses is only a caution.
#[test]
fn the_danger_is_shown_before_the_caution() {
    let mut h = Harness::new(PANEL);
    h.sign_with(SPOOF);
    h.advance_to_warnings();
    let texts = h.app.texts();
    let at = |s: &str| {
        texts
            .iter()
            .position(|t| t == s)
            .unwrap_or_else(|| panic!("{s:?} is not on the screen: {texts:?}"))
    };
    assert!(at(strings::EN.warn_change_spoof) < at(strings::EN.warn_address_reuse));
}

/// §4.4: "Sign with" is drawn only where more than one key could sign,
/// because a row of one chip offers nothing. With one key the hold is
/// live and there is no chip to turn it off with.
#[test]
fn one_signing_key_carries_no_chips() {
    let mut h = Harness::new(PANEL);
    h.sign_with(DEMO);
    h.advance_to_confirm();
    assert!(h.app.sign_enabled());
    assert!(
        h.app.rect_of(ids::at(ids::SIGN_KEY_BASE, 0)).is_none(),
        "one key needs no chip row"
    );
    assert!(
        !h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.sign_sign_with),
        "and no caption over it"
    );
}

/// A shell with no file to give leaves the scanner as it was, and the
/// row asks again. Back while waiting cancels the request, and a late
/// answer to a cancelled request is ignored.
#[test]
fn file_unavailable_leaves_the_scanner_and_the_row_asks_again() {
    let mut h = Harness::new(PANEL);
    h.with_key();
    h.open_sign_and_request();
    h.send(Event::FileUnavailable {
        kind: FileKind::Any,
    });
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.request_file();
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Wallet, "the scanner is left");
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    assert_eq!(h.app.sign_stage(), None);
    assert!(h.app.sign_inspection().is_none());
}

#[test]
fn a_cancelled_pick_returns_to_the_scanner_and_says_nothing() {
    let mut h = Harness::new(PANEL);
    h.with_key();
    let rows = {
        h.open_sign_and_request();
        h.send(Event::FileCancelled {
            kind: FileKind::Any,
        });
        h.app.texts()
    };
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    assert!(
        !rows.iter().any(|t| t == strings::EN.scan_waiting),
        "the person closed the picker; nothing is waiting: {rows:?}"
    );
    // A device with no file channel at all is back on the viewfinder,
    // where asking again is still offered.
    h.request_file();
    h.send(Event::FileUnavailable {
        kind: FileKind::Any,
    });
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    assert!(h.app.rect_of(ids::SCAN_FILE).is_some(), "asking again");
}

/// Bytes the scanner cannot place are the unknown menu; reading them as
/// a transaction anyway is the parse error, whose way out is off the
/// screen.
#[test]
fn garbage_read_as_a_transaction_shows_a_parse_error_with_back() {
    let mut h = Harness::new(PANEL);
    h.with_key();
    h.open_sign_and_request();
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: b"  hello, not a psbt \n".to_vec(),
    });
    h.tap(ids::SCAN_AS_PSBT);
    assert_eq!(h.app.sign_stage(), Some(Stage::ParseError));
    // "Try again" leaves the error, and the scanner is where a
    // transaction is read again.
    h.tap(ids::SIGN_BACK);
    assert_eq!(h.app.sign_stage(), None);
    // Binary with surrounding whitespace parses.
    let mut padded = b"\n  ".to_vec();
    padded.extend(demo_psbt().to_bytes());
    padded.extend(b"\n");
    h.open_sign_and_request();
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: padded,
    });
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Summary)));
}

#[test]
fn back_from_the_summary_drops_the_psbt_and_leaving_clears_everything() {
    let mut h = Harness::new(PANEL);
    h.sign_with(DEMO);
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Outputs)));
    h.tap(ids::BACK);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Summary)));
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(h.app.sign_inspection().is_none());
    assert_eq!(h.app.sign_stage(), None);
}

#[test]
fn selftest_passes_on_start_and_can_be_rerun() {
    let mut h = Harness::new(DisplayInfo {
        width: 320,
        height: 240,
        dpi: 143,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    });
    assert_eq!(h.app.selftest(), Some(Ok(osk_selftest::CHECKS.len())));
    assert_eq!(h.app.screen(), ScreenKind::Home);
    h.open_settings();
    h.tap(ids::SETTINGS_ABOUT_ROW);
    assert_eq!(h.app.screen(), ScreenKind::About);
    h.tap(ids::SETTINGS_SELFTEST);
    assert_eq!(h.app.selftest(), Some(Ok(osk_selftest::CHECKS.len())));
    assert_eq!(h.app.screen(), ScreenKind::About);
}

/// A save is not a save until the shell says the bytes are stored: the
/// file row waits for the answer, a second tap while it waits asks for
/// nothing more, and the name appears only once the file is there.
#[test]
fn a_save_waits_for_the_shell_before_it_names_the_file() {
    let mut h = Harness::new(PANEL);
    h.sign_with(DEMO);
    h.advance_to_confirm();
    h.hold(ids::SIGN_HOLD);

    let (kind, name_hint, _) = h.save();
    assert!(
        h.app.texts().iter().any(|t| t == strings::EN.sign_waiting),
        "the row waits: {:?}",
        h.app.texts()
    );
    let again = h.tap_commands(ids::SIGN_SAVE);
    assert!(
        !again.iter().any(|c| matches!(c, Command::WriteFile { .. })),
        "one write is out already: {again:?}"
    );

    h.send(Event::FileWritten { kind });
    let saved = strings::fill1(strings::EN.sign_saved_as, &name_hint);
    assert!(
        h.app.texts().contains(&saved),
        "the file is named: {:?}",
        h.app.texts()
    );
}

/// A shell that stored nothing — no card, no room, a picker the person
/// cancelled — leaves the row saying so and Save live, so the person can
/// put a card in and tap it again.
#[test]
fn a_save_the_shell_could_not_make_says_so_and_stays_offered() {
    let mut h = Harness::new(PANEL);
    h.sign_with(DEMO);
    h.advance_to_confirm();
    h.hold(ids::SIGN_HOLD);

    let (kind, name_hint, bytes) = h.save();
    h.send(Event::FileNotWritten { kind });
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.sign_not_saved),
        "the row says the file is not there: {:?}",
        h.app.texts()
    );
    let saved = strings::fill1(strings::EN.sign_saved_as, &name_hint);
    assert!(!h.app.texts().contains(&saved));

    // The same bytes go out again on a second tap.
    assert_eq!(h.save(), (kind, name_hint.clone(), bytes));
}

/// The Nonce setting decides the bytes the device signs with. Under Low
/// R every signature in the transaction the device writes is the
/// shortest form; under First they are the plain RFC 6979 ones, and for
/// this PSBT the two are not the same signature.
#[test]
fn the_nonce_setting_decides_the_signature_bytes() {
    let signed_under = |choice: usize| -> Vec<Vec<u8>> {
        let mut h = Harness::new(PANEL);
        h.open_settings();
        h.tap(ids::SETTINGS_NONCE_ROW);
        h.tap(ids::at(ids::SETTINGS_NONCE_BASE, choice));
        h.tap(ids::BACK);
        h.tap(ids::BACK);
        h.sign_with(DEMO);
        h.advance_to_confirm();
        h.hold(ids::SIGN_HOLD);
        let (_, _, bytes) = h.save();
        let hex = String::from_utf8(bytes).expect("hex");
        let tx: osk_psbt::bitcoin::Transaction =
            osk_psbt::bitcoin::consensus::encode::deserialize_hex(&hex).expect("transaction");
        tx.input
            .iter()
            .map(|i| i.witness.iter().next().expect("a signature").to_vec())
            .collect()
    };

    let low_r = signed_under(0);
    let first = signed_under(1);
    assert!(!low_r.is_empty());
    assert_eq!(low_r.len(), first.len());
    for sig in &low_r {
        let der = &sig[..sig.len() - 1];
        assert!(der.len() <= 70, "{} DER bytes under Low R", der.len());
    }
    assert_ne!(low_r, first, "the two settings signed the same bytes");
}

/// The review says what it made of the change, both ways: the amount and
/// the Mine badge when every change output came from a loaded key, the
/// danger state when one did not. A person who taps Sign and then holds
/// never opens an output screen, so the state has to be on the review.
#[test]
fn the_review_states_what_it_made_of_the_change() {
    let mine = OutputBadge::Mine.label();

    let mut h = Harness::new(TINY);
    h.sign_with(DEMO);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Summary)));
    let texts = h.app.texts();
    assert!(texts.iter().any(|t| t == mine), "{texts:?}");
    assert!(
        !texts
            .iter()
            .any(|t| t == strings::EN.sign_change_unverified),
        "{texts:?}"
    );

    let mut h = Harness::new(TINY);
    h.sign_with(UNVERIFIED);
    let texts = h.app.texts();
    assert!(
        texts
            .iter()
            .any(|t| t == strings::EN.sign_change_unverified),
        "{texts:?}"
    );
    assert!(!texts.iter().any(|t| t == mine), "{texts:?}");

    // Nothing comes back to this wallet: neither state is on the screen.
    let mut h = Harness::new(TINY);
    h.sign_with(DUST);
    assert_eq!(
        h.app
            .sign_inspection()
            .expect("inspected")
            .change_total
            .to_sat(),
        0
    );
    let texts = h.app.texts();
    assert!(!texts.iter().any(|t| t == mine), "{texts:?}");
    assert!(
        !texts
            .iter()
            .any(|t| t == strings::EN.sign_change_unverified),
        "{texts:?}"
    );
}

// ----- The transaction's Keys review (`docs/PLANNING.md` §16.110 rule 3) -----

/// The genesis coinbase, which names no key at all.
const GENESIS: &str = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000";

/// Scan is the one way in for anything read, and a PSBT goes to the
/// Sign review whether or not a key of it is loaded.
fn scan_transaction(h: &mut Harness, bytes: &[u8]) {
    h.go_home();
    h.tap(ids::HOME_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: bytes.to_vec(),
    });
}

/// A regtest device with nothing loaded, at the review of a transaction
/// whose key it does not hold.
fn review_without_the_key() -> Harness {
    let mut h = Harness::new(PHONE);
    h.set_network(3);
    scan_transaction(&mut h, DEMO);
    assert_eq!(h.app.screen(), ScreenKind::Sign);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Summary)));
    h
}

/// The review says which key it wants, that row is the review of the
/// transaction's keys, and a key loaded from it comes back to a review
/// that can go on.
#[test]
fn a_key_loaded_from_the_transactions_keys_comes_back_to_a_live_review() {
    let s = &strings::EN;
    let mut h = review_without_the_key();
    assert!(
        h.app.texts().iter().any(|t| t == s.sign_no_key),
        "the dead Continue says what to change: {:?}",
        h.app.texts()
    );
    assert!(
        h.app.rect_of(ids::SIGN_KEYS).is_some(),
        "the key context row opens the review: {:?}",
        h.app.texts()
    );

    h.tap(ids::SIGN_KEYS);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys);
    let texts = h.app.texts();
    assert!(texts.iter().any(|t| t == "73c5da0a"), "the key: {texts:?}");
    assert!(
        texts.iter().any(|t| t == s.key_not_loaded),
        "which this device has no key for: {texts:?}"
    );

    h.tap(ids::at(ids::SIGN_KEY_ROW_BASE, 0));
    assert_eq!(h.app.screen(), ScreenKind::Add, "the row opens Add a key");
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();

    assert_eq!(
        h.app.screen(),
        ScreenKind::WalletKeys,
        "the review is where the flow started"
    );
    assert!(
        !h.app.texts().iter().any(|t| t == s.key_not_loaded),
        "and the key is loaded now: {:?}",
        h.app.texts()
    );

    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Sign, "back at the review");
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Summary)));
    assert!(
        !h.app.texts().iter().any(|t| t == s.sign_no_key),
        "with nothing left to change: {:?}",
        h.app.texts()
    );
    assert!(
        h.app.texts().iter().any(|t| t == "73c5da0a"),
        "and the key named: {:?}",
        h.app.texts()
    );
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Outputs)));
}

/// A passphrase typed from that row must give the key the transaction
/// named: another key is stated and not kept.
#[test]
fn a_passphrase_typed_from_the_transactions_keys_must_give_that_key() {
    let s = &strings::EN;
    // Some other key is loaded, so that there is a key to open a
    // passphrase from; the transaction's own key is not.
    const ZOO: [&str; 12] = [
        "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "wrong",
    ];
    let mut h = Harness::new(PHONE);
    h.start_load(&ZOO);
    h.finish_load(None);
    let loaded = h.app.fingerprints().len();
    h.set_network(3);
    scan_transaction(&mut h, DEMO);
    assert_eq!(h.app.screen(), ScreenKind::Sign);

    h.tap(ids::SIGN_KEYS);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys);
    assert!(
        h.app.texts().iter().any(|t| t == s.key_not_loaded),
        "a key the transaction names and this device has not: {:?}",
        h.app.texts()
    );

    h.tap(ids::at(ids::SIGN_KEY_ROW_BASE, 0));
    assert_eq!(h.app.screen(), ScreenKind::Add);
    // The transaction is asking for one key, so Add a key offers the
    // way to it and asks which loaded key to start from.
    h.tap(ids::ADD_OPEN_PASSPHRASE);
    h.tap(ids::at(ids::PICK_KEY_BASE, 0));
    assert_eq!(h.app.screen(), ScreenKind::OpenPassphrase);
    h.type_text("winter");
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(
        h.app.screen(),
        ScreenKind::OpenPassphrase,
        "the wrong key is not added"
    );
    assert_eq!(h.app.fingerprints().len(), loaded, "and Keys is unchanged");
    let prefix = s.open_wrong_passphrase.split('{').next().expect("a prefix");
    assert!(
        h.app.texts().iter().any(|t| t.starts_with(prefix)),
        "both fingerprints, as a statement: {:?}",
        h.app.texts()
    );
}

/// Every key loaded: the review states them with the key glyph.
#[test]
fn a_transaction_whose_keys_are_all_loaded_shows_them_with_the_glyph() {
    let s = &strings::EN;
    let mut h = Harness::new(PHONE);
    h.sign_with(DEMO);
    h.tap(ids::SIGN_KEYS);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys);
    assert!(
        !h.app.texts().iter().any(|t| t == s.key_not_loaded),
        "nothing is missing: {:?}",
        h.app.texts()
    );
    let glyphs: Vec<_> = h
        .app
        .row_glyphs()
        .into_iter()
        .map(|(_, icon)| icon)
        .collect();
    assert!(
        glyphs.contains(&osk_ui::widgets::Icon::Wallet),
        "the key glyph on the row: {glyphs:?}"
    );
}

/// A transaction that names no key has no such row: there is nothing to
/// name. Reading one shows the row on the same terms as signing.
#[test]
fn a_transaction_that_names_no_key_has_no_keys_row() {
    let mut h = Harness::new(PHONE);
    h.open_tile(common::TILE_TOOLS);
    h.tap(ids::TOOLS_DECODE);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: GENESIS.as_bytes().to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Decode);
    assert!(
        h.app.rect_of(ids::SIGN_KEYS).is_none(),
        "no key row: {:?}",
        h.app.texts()
    );

    // The same reading flow over a transaction that does name a key
    // carries the row.
    let mut h = Harness::new(PHONE);
    h.set_network(3);
    h.open_tile(common::TILE_TOOLS);
    h.tap(ids::TOOLS_DECODE);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Decode);
    h.tap(ids::SIGN_KEYS);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys);
    assert!(
        h.app.texts().iter().any(|t| t == "73c5da0a"),
        "the key the transaction names: {:?}",
        h.app.texts()
    );
}

// ----- what a signature already on the transaction is checked for
// (`docs/PLANNING.md` §16.111) -----

/// The 2-of-3 wallet's transaction, which arrives unsigned, and the
/// wallet itself.
const TWO_OF_THREE: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.psbt");
const TWO_OF_THREE_POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.policy");
/// The "zoo … wrong" cosigner of that wallet, which is not this device.
const ZOO: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";

fn master_of(words: &str) -> MasterKey {
    let m = Mnemonic::parse(Language::English, words).unwrap();
    MasterKey::from_seed(&m.to_seed(b"").unwrap(), Network::Regtest)
}

/// The 2-of-3 transaction with the cosigner's signature on it, which is
/// what a coordinator hands this device when the other signer went
/// first.
fn cosigner_signed() -> Psbt {
    let zoo = master_of(ZOO);
    let keys = [KeyRef::from_master(&zoo, 0).unwrap()];
    let wallets = [osk_bip::policy::WalletPolicy::parse(TWO_OF_THREE_POLICY).unwrap()];
    let mut psbt = Psbt::parse_base64(TWO_OF_THREE.trim()).unwrap();
    osk_psbt::sign(
        &mut psbt,
        &[&zoo],
        &[zoo.fingerprint()],
        &Context {
            network: Network::Regtest,
            keys: &keys,
            wallets: &wallets,
            musig_session: None,
            shares: &[],
            carry: None,
        },
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
    )
    .expect("the cosigner signs its own wallet's transaction");
    psbt
}

impl Harness {
    /// Continue until the inputs step, which the Signatures row is on.
    fn advance_to_inputs(&mut self) {
        for _ in 0..8 {
            if self.app.sign_stage() == Some(Stage::Wizard(Step::Inputs)) {
                return;
            }
            self.tap(ids::SIGN_CONTINUE);
        }
        panic!("the inputs step was never reached");
    }
}

/// A signature another device made is checked here: the review counts
/// it, and the page it opens says it is this transaction's.
#[test]
fn the_signatures_page_says_a_cosigners_signature_verifies() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    h.sign_with(&cosigner_signed().to_bytes());
    h.advance_to_inputs();
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.sign_signatures),
        "the inputs review counts the signatures: {texts:?}"
    );

    h.tap(ids::SIGN_SIGNATURES);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Signatures)));
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.sign_sig_valid),
        "the cosigner's signature verifies: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.contains(&master_of(ZOO).fingerprint().to_string())),
        "and the row names the key that made it: {texts:?}"
    );
    // Nothing of this device's made it, so there is no nonce rule to
    // recompute it under.
    assert!(
        !texts
            .iter()
            .any(|t| t.contains(s.sign_sig_not_deterministic)),
        "another device's nonce is not this device's to judge: {texts:?}"
    );

    // The chevron goes back to the review it was opened from.
    h.tap(ids::BACK);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Inputs)));
}

/// One byte of that signature changed: the transaction was altered
/// after it was signed, and the review says so and shuts the hold.
#[test]
fn a_corrupted_signature_is_a_danger_the_review_stops_at() {
    let s = &strings::EN;
    let mut psbt = cosigner_signed();
    let input = &mut psbt.inner_mut().inputs[0];
    let (pk, sig) = input
        .partial_sigs
        .iter()
        .map(|(k, v)| (*k, *v))
        .next()
        .unwrap();
    let mut compact = sig.signature.serialize_compact();
    compact[40] ^= 0x01;
    input.partial_sigs.insert(
        pk,
        osk_psbt::bitcoin::ecdsa::Signature {
            signature: osk_psbt::bitcoin::secp256k1::ecdsa::Signature::from_compact(&compact)
                .unwrap(),
            sighash_type: sig.sighash_type,
        },
    );

    let mut h = Harness::new(PANEL);
    h.sign_with(&psbt.to_bytes());
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(
        insp.dangers()
            .iter()
            .any(|k| matches!(k, WarningKind::InvalidSignature { .. })),
        "{:?}",
        insp.warnings
    );
    h.advance_to_warnings();
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.sign_signatures),
        "the card is about the signatures: {texts:?}"
    );
    assert!(
        !h.app.sign_enabled(),
        "the hold is shut until it is accepted"
    );
}

/// Two signatures under one key sharing a nonce: the key follows from
/// them, and the review refuses the transaction until it is accepted.
#[test]
fn two_signatures_under_one_nonce_are_a_danger() {
    let mut psbt = cosigner_signed();
    // A second input of the same output, carrying a second signature
    // under the same key with the same `r`.
    let (pk, sig) = psbt.inner().inputs[0]
        .partial_sigs
        .iter()
        .map(|(k, v)| (*k, *v))
        .next()
        .unwrap();
    let mut compact = sig.signature.serialize_compact();
    // The same `r`, another `s`: one nonce, two signatures.
    compact[32..].copy_from_slice(&[1u8; 32]);
    let second = osk_psbt::bitcoin::ecdsa::Signature {
        signature: osk_psbt::bitcoin::secp256k1::ecdsa::Signature::from_compact(&compact).unwrap(),
        sighash_type: sig.sighash_type,
    };
    {
        let inner = psbt.inner_mut();
        let mut txin = inner.unsigned_tx.input[0].clone();
        txin.previous_output.vout += 1;
        inner.unsigned_tx.input.push(txin);
        let mut input = inner.inputs[0].clone();
        input.partial_sigs.clear();
        input.partial_sigs.insert(pk, second);
        inner.inputs.push(input);
    }

    let mut h = Harness::new(PANEL);
    h.sign_with(&psbt.to_bytes());
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(
        insp.dangers()
            .iter()
            .any(|k| matches!(k, WarningKind::NonceReuse { .. })),
        "{:?}",
        insp.warnings
    );
    h.advance_to_warnings();
    assert!(!h.app.sign_enabled(), "the hold is shut");
}

/// This device's own signature, once it is made, is on the same page,
/// named by the nonce rule the setting it was made under names.
#[test]
fn the_devices_own_signature_is_listed_under_the_nonce_rule_that_made_it() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    h.sign_with(DEMO);
    h.advance_to_confirm();
    h.hold(ids::SIGN_HOLD);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Result)));
    h.tap(ids::SIGN_SIGNATURES);
    let texts = h.app.texts();
    assert!(
        texts
            .iter()
            .any(|t| t.contains(s.sign_sig_valid) && t.contains(s.settings_nonce_low_r)),
        "the signature verifies and the setting's rule made it: {texts:?}"
    );
}

/// The key context row on the review states its key as a fingerprint:
/// the fingerprint glyph and then the eight characters (§16.131).
#[test]
fn the_reviews_key_row_puts_the_fingerprint_glyph_before_the_fingerprint() {
    let h = review_without_the_key();
    assert!(
        h.app.rect_of(ids::SIGN_KEYS).is_some(),
        "the review has its key row"
    );
    assert!(
        h.app
            .value_glyphs()
            .contains(&(String::from("73c5da0a"), Icon::Fingerprint)),
        "the key's fingerprint has no glyph: {:?}",
        h.app.value_glyphs()
    );
}
