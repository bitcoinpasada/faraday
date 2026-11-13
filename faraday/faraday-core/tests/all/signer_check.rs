//! Check a signer (`docs/SIGNER-CHECK.md` §3): the Tools tile opens it
//! with no key or wallet loaded; two PSBTs of one transaction in Files
//! open it closed on the default and checked; a PSBT the camera reads
//! goes to its card and nowhere else.
//!
//! The PSBTs are Bitcoin Core's own (`tools/vectors/psbt/`): the PSBT
//! sent, and the one Core signed.

use faraday_core::catalog::{Go, TILES};
use faraday_core::decode::check::{SignerPress, Slot};
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::strings::EN;
use faraday_core::testkit;
use faraday_core::testkit::Plain;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_shell_api::{App, Event};

fn vector(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../../tools/vectors/psbt/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn sent() -> Vec<u8> {
    vector("signer-core-sent.psbt")
}

fn signed() -> Vec<u8> {
    vector("signer-core-signed.psbt")
}

fn with_files(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(
            files.into_iter().map(|(n, b)| (n.to_string(), b)).collect(),
        ),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn open(app: &mut Faraday) {
    let tile = TILES
        .iter()
        .position(|t| t.go == Go::SignerCheck)
        .expect("the tile is in Tools") as u8;
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile));
}

fn texts(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts().plain()
}

fn shows(app: &mut Faraday, line: &str) -> bool {
    texts(app).iter().any(|t| t == line)
}

fn read(app: &mut Faraday, bytes: &[u8]) {
    app.event(Event::Scanned {
        bytes: bytes.to_vec(),
    });
    let _ = app.frame();
}

#[test]
fn the_tile_opens_with_no_key_or_wallet() {
    let mut app = testkit::started();
    open(&mut app);
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(shows(&mut app, EN.signer_sent));
    assert!(shows(&mut app, EN.signer_no_psbt));
}

#[test]
fn two_psbts_of_one_transaction_open_checked() {
    let mut app = with_files(vec![("sent.psbt", sent()), ("signed.psbt", signed())]);
    open(&mut app);
    assert!(shows(&mut app, EN.signer_only_signatures));
    // Both cards are closed on their choice, the fewer signatures Sent.
    let t = texts(&mut app);
    assert!(t.iter().any(|l| l == "sent.psbt"));
    assert!(t.iter().any(|l| l == "signed.psbt"));
}

#[test]
fn change_opens_a_card_and_a_new_choice_checks_again() {
    let mut app = with_files(vec![("sent.psbt", sent()), ("signed.psbt", signed())]);
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Card(0)));
    assert!(!shows(&mut app, EN.signer_only_signatures));
    // The signed file as Sent leaves the Returned card open, listing the
    // other file.
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Sent, 1)));
    assert!(!shows(&mut app, EN.signer_only_signatures));
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Returned, 0)));
    // The signature is gone from what came back.
    let t = texts(&mut app);
    assert!(!t.iter().any(|l| l == EN.signer_only_signatures));
    assert!(
        t.iter()
            .any(|l| l.ends_with(" finding") || l.ends_with(" findings")),
        "{t:?}"
    );
}

#[test]
fn a_scanned_psbt_reaches_its_card_and_nothing_else() {
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Sent)));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    read(&mut app, &sent());
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(app.inbox().is_empty(), "nothing was filed");
    assert!(shows(&mut app, EN.signer_scanned));
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Returned)));
    read(&mut app, &signed());
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(app.inbox().is_empty());
    assert!(shows(&mut app, EN.signer_only_signatures));
}

#[test]
fn something_else_scanned_is_not_a_psbt() {
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Returned)));
    read(&mut app, b"hello");
    assert_eq!(app.sheet, Some(Sheet::Scan));
    let note = app
        .scan()
        .and_then(|s| s.note.as_ref().map(|n| String::from(n.as_str())))
        .unwrap_or_default();
    assert_eq!(note, "Not a PSBT");
    assert!(app.inbox().is_empty());
}

fn pair() -> Faraday {
    let mut app = with_files(vec![("sent.psbt", sent()), ("signed.psbt", signed())]);
    open(&mut app);
    app
}

/// The drawn lines that hold `part`.
fn holding(app: &mut Faraday, part: &str) -> Vec<String> {
    texts(app)
        .into_iter()
        .filter(|t| t.contains(part))
        .collect()
}

#[test]
fn swapping_a_clean_pair_says_each_signature_is_gone() {
    let mut app = pair();
    assert!(shows(&mut app, EN.signer_only_signatures));
    app.press(Action::SignerCheck(SignerPress::Swap));
    assert!(!shows(&mut app, EN.signer_only_signatures));
    let gone = holding(&mut app, "was removed");
    assert!(!gone.is_empty(), "{:?}", texts(&mut app));
    assert!(gone.iter().all(|l| l.starts_with("Caution")), "{gone:?}");
    assert!(!holding(&mut app, EN.signer_s_none_added).is_empty());
}

#[test]
fn choose_again_opens_the_returned_card() {
    let mut app = pair();
    app.press(Action::SignerCheck(SignerPress::Again));
    assert!(!shows(&mut app, EN.signer_only_signatures));
    assert!(shows(&mut app, EN.signer_scan));
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Returned, 1)));
    assert!(shows(&mut app, EN.signer_only_signatures));
}

#[test]
fn two_transactions_are_not_the_same_and_choose_again_reopens_returned() {
    let other = vector("wallet-tapmulti-signed.psbt");
    let mut app = with_files(vec![("sent.psbt", sent()), ("other.psbt", other)]);
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Sent, 0)));
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Returned, 1)));
    assert!(shows(&mut app, EN.signer_not_the_same));
    assert!(!shows(&mut app, EN.signer_findings_label));
    app.press(Action::SignerCheck(SignerPress::Again));
    assert!(!shows(&mut app, EN.signer_not_the_same));
}

#[test]
fn show_as_qr_carries_the_returned_psbt_and_after_swap_the_other() {
    let raw =
        |v: Vec<u8>| faraday_psbt::base64::decode(std::str::from_utf8(&v).unwrap().trim()).unwrap();
    let carried = |app: &Faraday| match &app.qr().expect("the sheet's code").source {
        faraday_qr::send::Source::Psbt(b) => b.clone(),
        _ => panic!("not a PSBT"),
    };
    let mut app = pair();
    app.press(Action::SignerCheck(SignerPress::Qr));
    assert_eq!(app.sheet, Some(Sheet::Qr));
    assert_eq!(carried(&app), raw(signed()));
    app.press(Action::Cancel);
    app.press(Action::SignerCheck(SignerPress::Swap));
    app.press(Action::SignerCheck(SignerPress::Qr));
    assert_eq!(carried(&app), raw(sent()));
}

#[test]
fn done_returns_to_where_the_flow_was_opened() {
    let mut app = pair();
    app.press(Action::SignerCheck(SignerPress::Back));
    assert_eq!(app.screen, Screen::Catalog);
}

#[test]
fn a_proprietary_field_is_named_by_its_sizes() {
    let mut app = with_files(vec![
        ("sent.psbt", sent()),
        ("device.psbt", vector("signer-proprietary-added.psbt")),
    ]);
    open(&mut app);
    let line = holding(&mut app, "proprietary field added");
    assert_eq!(line.len(), 1, "{:?}", texts(&mut app));
    assert!(line[0].starts_with("Danger"), "{line:?}");
    assert!(
        line[0].contains("prefix 6 bytes · subtype 1 · value 4 bytes"),
        "{line:?}"
    );
}

#[test]
fn findings_read_blocked_then_danger_then_caution() {
    let c = testkit::compute();
    let text = String::from_utf8(vector("signer-proprietary-added.psbt")).expect("a text file");
    let mut changed = faraday_psbt::Psbt::parse_base64(&c, text.trim()).expect("a PSBT");
    changed.inner_mut().unsigned_tx.output[0].value -= faraday_bip::bitcoin::Amount::from_sat(1);
    changed.inner_mut().inputs[0].bip32_derivation.clear();
    let mut app = with_files(vec![
        ("sent.psbt", sent()),
        ("changed.psbt", changed.to_bytes()),
    ]);
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Sent, 0)));
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Returned, 1)));
    // The Result scrolls: what the top shows, then what is below it.
    let mut t = texts(&mut app);
    app.event(Event::Wheel {
        x: 640,
        y: 400,
        dy: 3000,
    });
    app.settle();
    for l in texts(&mut app) {
        if !t.contains(&l) {
            t.push(l);
        }
    }
    let first = |word: &str| t.iter().position(|l| l.starts_with(word));
    let last = |word: &str| t.iter().rposition(|l| l.starts_with(word));
    let (blocked, danger, caution) = (first("Blocked"), first("Danger"), first("Caution"));
    assert!(
        blocked.is_some() && danger.is_some() && caution.is_some(),
        "{t:?}"
    );
    assert!(
        last("Blocked") < danger && last("Danger") < caution,
        "{t:?}"
    );
}

#[test]
fn a_locked_session_holds_nothing_of_the_flow() {
    let mut app = pair();
    assert!(app.signer_check_kept());
    app.press(Action::Lock);
    assert!(!app.signer_check_kept());
}

const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                       abandon abandon about";

/// Sent is the Core-regtest PSBT, Returned the one the "abandon" key
/// signed with a random nonce, both in Files; the pair opens checked.
fn random_nonce_pair() -> Faraday {
    let mut app = with_files(vec![
        ("sent.psbt", vector("demo-regtest.psbt")),
        ("random.psbt", vector("signer-random-nonce.psbt")),
    ]);
    open(&mut app);
    app
}

/// A press that leaves the screen as it is: the result card opened.
fn touch(app: &mut Faraday) {
    app.press(Action::SignerCheck(SignerPress::Card(2)));
}

#[test]
fn loading_the_key_after_a_check_turns_the_result_to_a_finding_and_dropping_it_takes_it_back() {
    let mut app = random_nonce_pair();
    assert!(
        !holding(&mut app, EN.signer_nonces_recomputed).is_empty(),
        "{:?}",
        texts(&mut app)
    );
    assert!(holding(&mut app, "could not be reproduced").is_empty());
    assert!(shows(&mut app, EN.signer_only_signatures));

    let c = faraday_core::testkit::compute();
    app.session.keys.add_words(&c, ABANDON, "", None).unwrap();
    touch(&mut app);
    let found = holding(&mut app, "could not be reproduced");
    assert_eq!(found.len(), 1, "{:?}", texts(&mut app));
    assert!(found[0].starts_with("Danger"), "{found:?}");
    assert!(!shows(&mut app, EN.signer_only_signatures));
    assert!(holding(&mut app, "not loaded").is_empty());

    app.session.keys.remove(0);
    touch(&mut app);
    assert!(holding(&mut app, "could not be reproduced").is_empty());
    assert!(shows(&mut app, EN.signer_only_signatures));
}

#[test]
fn the_nonce_fact_says_none_for_a_key_not_loaded_and_how_many_for_one_that_is() {
    let mut app = random_nonce_pair();
    let none = holding(&mut app, "not loaded");
    assert_eq!(none.len(), 1, "{:?}", texts(&mut app));
    assert!(none[0].starts_with("None: "), "{none:?}");
    let c = faraday_core::testkit::compute();
    app.session.keys.add_words(&c, ABANDON, "", None).unwrap();
    touch(&mut app);
    assert!(holding(&mut app, "not loaded").is_empty());
    // The signature is told from a recomputed nonce: none of one.
    assert!(shows(&mut app, "0 of 1"), "{:?}", texts(&mut app));
}

/// From Add a key's Key card: Lock in, Continue to Words, the quiz
/// passed, and Done.
fn finish_key(app: &mut Faraday) {
    use faraday_core::quiz::QuizState;
    app.press(Action::KLock);
    app.press(Action::KNext);
    for i in 0..4u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x50 + i; 32],
        )));
    }
    app.press(Action::KNext);
    for _ in 0..30 {
        let Some(q) = app.keygen().and_then(|k| k.quiz.as_ref()) else {
            break;
        };
        if q.state() == QuizState::Passed {
            break;
        }
        let slot = q.correct_slot() as u8;
        app.press(Action::KQuiz(slot));
    }
    app.press(Action::KAdd);
}

#[test]
fn load_opens_add_a_key_and_its_end_comes_back_to_the_result_with_the_key_loaded() {
    use faraday_core::keygen::Start;
    let mut app = random_nonce_pair();
    let _ = app.frame();
    assert!(
        !holding(&mut app, "Load ").is_empty(),
        "{:?}",
        texts(&mut app)
    );
    app.press(Action::SignerCheck(SignerPress::Load(0)));
    assert_eq!(app.screen, Screen::KeyGen);
    // The seed scanned in Add a key: "abandon" eleven times and "about".
    app.press(Action::KStart(Start::Scan.index()));
    read(
        &mut app,
        b"000000000000000000000000000000000000000000000003",
    );
    finish_key(&mut app);
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(!holding(&mut app, "could not be reproduced").is_empty());
    assert!(holding(&mut app, "Load ").is_empty(), "no key left to load");
}

#[test]
fn a_key_loaded_by_an_input_that_is_no_press_makes_the_kept_report_again() {
    let mut app = random_nonce_pair();
    assert!(shows(&mut app, EN.signer_only_signatures));
    let c = faraday_core::testkit::compute();
    app.session.keys.add_words(&c, ABANDON, "", None).unwrap();
    // A code read where no scanner is open: no press, and no change of
    // screen, but the report follows the keys.
    read(&mut app, b"x");
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(!holding(&mut app, "could not be reproduced").is_empty());
}

#[test]
fn change_reads_verified_only_with_change_on_the_change_chain_of_a_wallet_loaded() {
    let mut app = pair();
    assert!(shows(&mut app, EN.signer_change_not_verified));
    assert!(!shows(&mut app, EN.signer_change_verified));
    // A wallet loaded while the report is kept: the report follows it.
    let text = String::from_utf8(vector("wallet-2of3.txt")).unwrap();
    let c = faraday_core::testkit::compute();
    app.session
        .add_wallet(&c, "w", &text, "test", faraday_core::wallet::Origin::Named)
        .unwrap();
    app.session
        .keys
        .add_words(
            &c,
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
            "",
            None,
        )
        .unwrap();
    touch(&mut app);
    assert!(
        shows(&mut app, EN.signer_change_verified),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn a_finalized_pair_defaults_and_reads_clean() {
    let mut app = with_files(vec![
        ("final.psbt", vector("signer-core-final.psbt")),
        ("sent.psbt", vector("demo-regtest.psbt")),
    ]);
    open(&mut app);
    assert!(
        shows(&mut app, EN.signer_only_one_finalized),
        "{:?}",
        texts(&mut app)
    );
    app.press(Action::SignerCheck(SignerPress::Card(1)));
    assert!(
        holding(&mut app, "1 signature ").len() == 1,
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn a_taproot_pair_defaults_and_reads_clean() {
    let mut app = with_files(vec![
        ("first.psbt", vector("wallet-tapmulti-first.psbt")),
        ("signed.psbt", vector("wallet-tapmulti-signed.psbt")),
    ]);
    open(&mut app);
    assert!(
        texts(&mut app).iter().any(|l| l == "first.psbt"),
        "{:?}",
        texts(&mut app)
    );
    assert!(holding(&mut app, "0 signatures").is_empty());
    assert!(
        holding(&mut app, "Danger").is_empty(),
        "{:?}",
        texts(&mut app)
    );
    assert!(
        shows(&mut app, EN.signer_only_signatures),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn a_file_replaced_in_files_marks_the_closed_card_and_keeps_the_copy_and_report() {
    let mut app = pair();
    assert!(shows(&mut app, EN.signer_only_signatures));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("sent.psbt"), sent()),
            (
                faraday_text::Text::of("signed.psbt"),
                vector("wallet-tapmulti-signed.psbt"),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let line = EN.signer_file_differs.replace("{}", "signed.psbt");
    assert!(shows(&mut app, &line), "{:?}", texts(&mut app));
    assert!(
        shows(&mut app, EN.signer_only_signatures),
        "the report is kept"
    );

    // The replacement is a file of its own: the Sent card lists it, and
    // picking it again clears the mark.
    app.press(Action::SignerCheck(SignerPress::Card(0)));
    let listed = texts(&mut app)
        .iter()
        .filter(|l| *l == "signed.psbt")
        .count();
    assert_eq!(listed, 1, "{:?}", texts(&mut app));
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Sent, 0)));
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Returned, 1)));
    assert!(!shows(&mut app, &line), "{:?}", texts(&mut app));
}

#[test]
fn a_file_written_to_the_stick_or_taken_out_is_judged_by_both_boxes() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("sent.psbt"), sent()),
            (faraday_text::Text::of("signed.psbt"), signed()),
        ]),
        outbox: faraday_core::files_from(vec![(faraday_text::Text::of("signed.psbt"), signed())]),
        kept: Vec::new(),
    });
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Sent, 0)));
    // Files lists From the stick, then For the stick: the last is the
    // outbox copy.
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Returned, 2)));
    assert!(
        shows(&mut app, EN.signer_only_signatures),
        "{:?}",
        texts(&mut app)
    );
    let line = EN.signer_file_differs.replace("{}", "signed.psbt");
    // Written: it leaves For the stick, and the same file is in From.
    app.storage(StorageEvent::Written {
        stick: faraday_text::Text::of("s"),
        name: faraday_text::Text::of("signed.psbt"),
        wrote_as: faraday_text::Text::of("signed.psbt"),
    });
    assert!(!shows(&mut app, &line), "{:?}", texts(&mut app));
    // Taken out of both boxes: marked.
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(faraday_text::Text::of("sent.psbt"), sent())]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    assert!(shows(&mut app, &line), "{:?}", texts(&mut app));
}

#[test]
fn an_empty_flow_kept_does_not_block_the_default_pair() {
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Back));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("sent.psbt"), sent()),
            (faraday_text::Text::of("signed.psbt"), signed()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    open(&mut app);
    assert!(
        shows(&mut app, EN.signer_only_signatures),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn a_signature_in_two_places_gives_no_invented_order() {
    use faraday_psbt::Psbt;
    let c = faraday_core::testkit::compute();
    let parse = |name: &str| {
        Psbt::parse_base64(&c, String::from_utf8(vector(name)).unwrap().trim()).unwrap()
    };
    let done = parse("signer-core-final.psbt");
    let mut both = parse("signer-core-signed.psbt");
    both.inner_mut().inputs[0].final_script_witness =
        done.inner().inputs[0].final_script_witness.clone();
    let mut app = with_files(vec![
        ("both.psbt", both.to_bytes()),
        ("final.psbt", done.to_bytes()),
    ]);
    open(&mut app);
    // One signature each: neither is the fewer, so both cards are open.
    assert!(
        !shows(&mut app, EN.signer_only_one_finalized),
        "{:?}",
        texts(&mut app)
    );
    assert!(shows(&mut app, EN.signer_no_psbt) || shows(&mut app, EN.signer_scan));
    assert!(!shows(&mut app, EN.signer_only_signatures));
}

#[test]
fn the_flow_listing_a_scanned_junk_leaves_no_refusal_after_a_reopen() {
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Returned)));
    read(&mut app, b"hello");
    // The scanner stated it; a good read of the other card closes it.
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Sent)));
    read(&mut app, &sent());
    assert_eq!(app.sheet, None);
    assert!(
        !shows(&mut app, EN.wallet_not_a_psbt),
        "{:?}",
        texts(&mut app)
    );
}

/// The unsigned spending PSBT and `returned` (the same transaction with
/// other output notes), with the spending wallet and its key loaded, as
/// Sent and Returned.
fn spending_pair(returned: impl Fn(&mut faraday_psbt::Psbt)) -> Faraday {
    use faraday_core::testkit::Kit;
    let k: Kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = testkit::unsigned(&k).unwrap();
    let mut other = psbt.clone();
    returned(&mut other);
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (
                faraday_text::Text::of("spending-wallet.txt"),
                k.descriptor.into_bytes(),
            ),
            (faraday_text::Text::of("a.psbt"), psbt.to_bytes()),
            (faraday_text::Text::of("b.psbt"), other.to_bytes()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    crate::common::add_key(&mut app, "bacon");
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Sent, 0)));
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Returned, 1)));
    app
}

#[test]
fn change_off_the_change_chain_or_beside_change_that_cannot_be_verified_reads_not_verified() {
    use faraday_psbt::bitcoin::bip32::{ChildNumber, DerivationPath, Fingerprint};
    let mut app = spending_pair(|_| {});
    assert!(
        shows(&mut app, EN.signer_change_verified),
        "{:?}",
        texts(&mut app)
    );

    // The change output filed on the receive chain.
    let mut app = spending_pair(|p| {
        for (_, path) in p.inner_mut().outputs[1].bip32_derivation.values_mut() {
            let mut steps: Vec<ChildNumber> = path.into_iter().copied().collect();
            let n = steps.len();
            steps[n - 2] = ChildNumber::Normal { index: 0 };
            *path = DerivationPath::from(steps);
        }
    });
    assert!(
        shows(&mut app, EN.signer_change_not_verified),
        "{:?}",
        texts(&mut app)
    );

    // Verified change beside an output that claims a key not loaded.
    let mut app = spending_pair(|p| {
        let mut claim = p.inner().outputs[1].bip32_derivation.clone();
        for (fp, _) in claim.values_mut() {
            *fp = Fingerprint::from([9, 9, 9, 9]);
        }
        p.inner_mut().outputs[0].bip32_derivation = claim;
    });
    assert!(
        shows(&mut app, EN.signer_change_not_verified),
        "{:?}",
        texts(&mut app)
    );
}

fn menu_of(app: &mut Faraday, row: Row) -> Vec<(String, Action)> {
    let _ = app.frame();
    app.press(Action::File(FilePress::Menu(row)));
    assert_eq!(app.sheet, Some(Sheet::FileMenu));
    app.files_menu()
        .menu
        .as_ref()
        .expect("its menu")
        .lines
        .clone()
}

fn choose_check(app: &mut Faraday, row: Row) {
    let lines = menu_of(app, row);
    let (_, a) = lines
        .iter()
        .find(|(l, _)| l == EN.catalog_check_a_signer)
        .unwrap_or_else(|| panic!("Check a signer is in {lines:?}"));
    let _ = app.frame();
    app.press(*a);
}

#[test]
fn a_psbt_from_the_stick_with_a_pair_opens_the_flow_checked_and_done_returns_to_files() {
    let mut app = with_files(vec![("sent.psbt", sent()), ("signed.psbt", signed())]);
    app.press(Action::Nav(Screen::Files));
    choose_check(&mut app, Row::In(1));
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(shows(&mut app, EN.signer_only_signatures));
    app.press(Action::SignerCheck(SignerPress::Back));
    assert_eq!(app.screen, Screen::Files);
}

#[test]
fn a_psbt_from_the_stick_with_no_pair_opens_on_the_sent_card() {
    let other = vector("wallet-tapmulti-signed.psbt");
    let mut app = with_files(vec![("signed.psbt", signed()), ("other.psbt", other)]);
    app.press(Action::Nav(Screen::Files));
    choose_check(&mut app, Row::In(0));
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(!shows(&mut app, EN.signer_only_signatures));
    let t = texts(&mut app);
    assert!(
        t.iter().any(|l| l == "signed.psbt"),
        "Returned, closed: {t:?}"
    );
    assert!(t.iter().any(|l| l == "other.psbt"), "Sent's rows: {t:?}");
    assert!(t.iter().any(|l| l == EN.signer_scan));
}

#[test]
fn the_entry_follows_show_as_qr_on_a_file_for_the_stick_and_checks_the_pair() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("sent.psbt"), sent()),
            (faraday_text::Text::of("signed.psbt"), signed()),
        ]),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Files));
    let words: Vec<String> = menu_of(&mut app, Row::Out(1))
        .into_iter()
        .map(|(l, _)| l)
        .collect();
    let at = words
        .iter()
        .position(|w| w == EN.catalog_check_a_signer)
        .unwrap_or_else(|| panic!("{words:?}"));
    if let Some(qr) = words.iter().position(|w| w == EN.files_show_as_qr) {
        assert_eq!(at, qr + 1, "{words:?}");
    }
    app.press(Action::Cancel);
    choose_check(&mut app, Row::Out(1));
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(shows(&mut app, EN.signer_only_signatures));
}

#[test]
fn from_files_the_one_other_file_with_fewer_signatures_is_sent() {
    // Unsigned, and two signed by two devices (one signature each).
    let mut app = with_files(vec![
        ("unsigned.psbt", sent()),
        ("a.psbt", signed()),
        ("b.psbt", vector("signer-sig-changed.psbt")),
    ]);
    app.press(Action::Nav(Screen::Files));
    choose_check(&mut app, Row::In(1));
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(
        shows(&mut app, EN.signer_only_signatures),
        "{:?}",
        texts(&mut app)
    );
    assert!(texts(&mut app).iter().any(|l| l == "unsigned.psbt"));
}

#[test]
fn the_tile_reopens_a_kept_flow_as_it_was() {
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Sent)));
    read(&mut app, &sent());
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Returned)));
    read(&mut app, &signed());
    assert!(shows(&mut app, EN.signer_only_signatures));
    app.press(Action::SignerCheck(SignerPress::Back));
    assert_eq!(app.screen, Screen::Catalog);
    open(&mut app);
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(shows(&mut app, EN.signer_only_signatures));
    let t = texts(&mut app);
    assert_eq!(
        t.iter().filter(|l| *l == EN.signer_scanned).count(),
        2,
        "{t:?}"
    );
    app.press(Action::SignerCheck(SignerPress::Back));
    assert_eq!(app.screen, Screen::Catalog);
}

#[test]
fn a_returned_card_with_only_the_sent_file_offers_scan_and_no_empty_line() {
    let mut app = with_files(vec![("sent.psbt", sent())]);
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Sent, 0)));
    assert!(shows(&mut app, EN.signer_scan));
    assert!(!shows(&mut app, EN.signer_no_psbt));
}
