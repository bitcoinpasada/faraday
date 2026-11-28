//! The secrets part: what a text or a file holds that may not leave as it is, and the sheet that
//! sends a secret where the person says.

use faraday_app_test::{TestView, seen, test_compute};
use faraday_kernel::session::FileKind;
use faraday_kernel::{AtLock, Message};
use faraday_port::bytes::Bytes;
use faraday_text::Text;

use crate::{
    Answer, Exposure, Finished, MAX_TEXT, Msg, Press, SecretKind, SecretOut, State, TextCheck,
    Then, check, classify, exposure, update,
};

/// BIP 39's first test vector.
const WORDS: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                     abandon abandon about";
/// BIP 32's first test vector, the master private key.
const XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";
/// A WIF private key from the Bitcoin wiki.
const WIF: &str = "5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ";

fn checked(text: &str) -> TextCheck {
    check(test_compute(), text)
}

/// Invariant: A text that is a seed's words, an extended private key or a private key is refused and named by its kind; an ordinary line, an empty text and a 64-digit hash are not refused.
/// Regression: Transfer's Send text or My seed's Compare lets a seed or a key out, or refuses a transaction id.
/// Owner: app
#[test]
fn a_text_that_is_a_secret_is_refused_and_named_by_its_kind() {
    assert_eq!(checked(WORDS), TextCheck::Secret(SecretKind::SeedWords));
    assert_eq!(checked(XPRV), TextCheck::Secret(SecretKind::Xprv));
    assert_eq!(checked(WIF), TextCheck::Secret(SecretKind::Wif));
    assert_eq!(checked(""), TextCheck::Fine);
    assert_eq!(checked("  \n "), TextCheck::Fine);
    assert_eq!(
        checked("lunch at noon, send it to the bakery"),
        TextCheck::Fine
    );
    assert_eq!(checked(&"ab".repeat(32)), TextCheck::Fine);
    assert_eq!(checked(&"ab".repeat(20)), TextCheck::SeedHex);
}

/// Invariant: A secret inside other text is found: a key after a label, a seed among words, an armoured private key; the key among other words is inside the text, not the text.
/// Regression: A seed or a key is sent along with a sentence about it.
/// Owner: app
#[test]
fn a_secret_inside_other_text_is_found() {
    let seed = format!("my backup: {WORDS} keep it safe");
    assert!(matches!(
        checked(&seed),
        TextCheck::Secret(SecretKind::SeedWords) | TextCheck::Inside(SecretKind::SeedWords)
    ));
    assert!(matches!(
        checked(&format!("key = {WIF}")),
        TextCheck::Secret(_) | TextCheck::Inside(_)
    ));
    assert_eq!(
        checked(&format!("{XPRV} and more")),
        TextCheck::Inside(SecretKind::Xprv)
    );
    // Written in pieces: the armour's own words are what a secret scan looks for.
    let armoured = format!("-----{} OPENSSH {} KEY-----\nabc\n", "BEGIN", "PRIVATE");
    assert_eq!(checked(&armoured), TextCheck::Inside(SecretKind::Wif));
}

/// Invariant: A text over the most is too long, tested before any secret scan, so an over-long secret is too long and never read; zero-width characters do not hide a seed.
/// Regression: A long text is scanned for ever, or a seed with a zero-width space between its words passes.
/// Owner: app
#[test]
fn an_over_long_text_is_too_long_and_invisible_characters_hide_nothing() {
    let long = format!("{WORDS} {}", "x".repeat(MAX_TEXT));
    assert_eq!(checked(&long), TextCheck::TooLong);
    let hidden = WORDS.replace(' ', " \u{200B}");
    assert_eq!(checked(&hidden), TextCheck::Secret(SecretKind::SeedWords));
}

/// Invariant: Each file kind keeps its exposure: a vault, a KeePass database and a Krux code are sealed; words, entries, a seed part, a key line, a wallet with secrets and a Core wallet are secret; the rest are public.
/// Regression: A secret file kind is listed public and goes out plain, or a public one is held back.
/// Owner: app
#[test]
fn each_file_kind_keeps_its_exposure() {
    use FileKind as F;
    for kind in [F::Vault, F::Kdbx, F::KdbxOther, F::KruxCode] {
        assert_eq!(exposure(kind), Exposure::Sealed, "{kind:?}");
    }
    for kind in [
        F::Entries,
        F::Words,
        F::SeedPart,
        F::KeyLine,
        F::SecretWallet,
        F::KeyWithSecret,
        F::CoreWallet,
    ] {
        assert_eq!(exposure(kind), Exposure::Secret, "{kind:?}");
    }
    for kind in [
        F::Psbt,
        F::Wallet,
        F::Transaction,
        F::Sheet,
        F::Message,
        F::Key,
        F::Share,
        F::EfiImage,
        F::Pdf,
        F::Text,
        F::Gpg,
        F::Other,
    ] {
        assert_eq!(exposure(kind), Exposure::Public, "{kind:?}");
    }
}

/// Invariant: A file holding a seed's words or an extended private key classifies as a secret kind wherever it sits; plain text, a PDF, a Core wallet database and an EFI image classify as what they are.
/// Regression: A seed in a note file is listed as plain text and saved unprotected without the warning.
/// Owner: app
#[test]
fn a_file_with_a_secret_is_classified_as_one() {
    let c = test_compute();
    assert_eq!(classify(c, "a.txt", WORDS.as_bytes()), FileKind::Words);
    assert_eq!(
        classify(c, "a.txt", format!("{WORDS}\n").as_bytes()),
        FileKind::Words
    );
    assert_eq!(classify(c, "k.txt", XPRV.as_bytes()), FileKind::KeyLine);
    assert_eq!(classify(c, "n.txt", b"buy milk, call mum"), FileKind::Text);
    assert_eq!(classify(c, "a.pdf", b"%PDF-1.4 ..."), FileKind::Pdf);
    assert_eq!(
        classify(c, "w", b"SQLite format 3\0..."),
        FileKind::CoreWallet
    );
    assert_eq!(classify(c, "BOOT.EFI", b"MZ\0\0"), FileKind::EfiImage);
    assert_eq!(classify(c, "x.bin", &[0, 159, 146, 150]), FileKind::Other);
}

fn secret() -> SecretOut {
    SecretOut::new(
        "seed.txt",
        Bytes::new(WORDS.as_bytes().to_vec()),
        "a seed",
        "all of it",
    )
}

/// Invariant: A secret waiting on the sheet is held as typed until a lock, which drops it; a secret for an Inbox file on its way to another stick holds nothing, since the Inbox already holds it.
/// Regression: A lock leaves the secret on the sheet in the app's state, or a stick visit is held open for a copy the Inbox holds.
/// Owner: app
#[test]
fn a_lock_drops_the_secret_waiting_on_the_sheet() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = State::new();
    let _ = seen(update(&mut state, Msg::Offer(secret()), &cx));
    assert!(state.holds().typed && state.out().is_some());
    let _ = seen(update(
        &mut state,
        Msg::UnprotectedOn(Text::of("stick")),
        &cx,
    ));
    assert!(state.plain().is_some());
    state.at_lock();
    assert!(!state.holds().typed && state.out().is_none() && state.plain().is_none());

    let mut visit = secret();
    visit.to_visit = true;
    let _ = seen(update(&mut state, Msg::Offer(visit), &cx));
    assert!(!state.holds().typed);
}

/// Invariant: A secret goes to the Outbox as a plain file only after the hold: nothing is sent without the ask, the ask names the files, a hold for another feature's ask hands it over with the secret still waiting, and a Cancel drops the secret and says which plan to undo.
/// Regression: A secret reaches the Outbox unprotected without the hold, or a Cancel leaves it waiting.
/// Owner: app
#[test]
fn a_secret_goes_out_unprotected_only_past_the_hold() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = State::new();
    let _ = seen(update(&mut state, Msg::Offer(secret()), &cx));
    // Without the ask nothing is sent.
    let fx = seen(update(&mut state, Msg::Press(Press::Hold), &cx));
    assert!(fx.changes.is_empty() && state.out().is_some());
    let _ = seen(update(
        &mut state,
        Msg::UnprotectedOn(Text::of("stick")),
        &cx,
    ));
    assert_eq!(state.plain().map(|p| p.then), Some(Then::Secret));
    assert_eq!(
        state.plain().map(|p| p.name.as_str().to_string()),
        Some("seed.txt".to_string())
    );

    // A hold for another feature's ask is handed to its owner with the secret still waiting.
    let _ = seen(update(
        &mut state,
        Msg::AskPlain(crate::Plain {
            name: Text::of("plan"),
            on: Text::of("stick"),
            keep: Text::of("stick"),
            then: Then::Lock,
        }),
        &cx,
    ));
    let fx = seen(update(&mut state, Msg::Press(Press::Hold), &cx));
    assert!(fx.changes.is_empty() && state.out().is_some());
    assert_eq!(state.take_finished(), Some(Finished::Held(Then::Lock)));

    let _ = seen(update(&mut state, Msg::Offer(secret()), &cx));
    let _ = seen(update(
        &mut state,
        Msg::AskPlain(crate::Plain {
            name: Text::of("plan"),
            on: Text::of("stick"),
            keep: Text::of("stick"),
            then: Then::Plan,
        }),
        &cx,
    ));
    let _ = seen(update(&mut state, Msg::Press(Press::Cancel), &cx));
    assert!(state.out().is_none() && state.plain().is_none());
    assert_eq!(
        state.take_finished(),
        Some(Finished::Cancelled(Some(Then::Plan)))
    );
}

/// Invariant: A secret for another stick's visit is ticked on the visit and goes nowhere else; with no vault open a press on the vault changes nothing and leaves the secret waiting.
/// Regression: A visit's file reaches the Outbox, or a press with no vault open drops the secret.
/// Owner: app
#[test]
fn a_visit_file_is_ticked_and_no_vault_leaves_the_secret_waiting() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = State::new();
    let mut visit = secret();
    visit.to_visit = true;
    let _ = seen(update(&mut state, Msg::Offer(visit), &cx));
    let _ = seen(update(
        &mut state,
        Msg::UnprotectedOn(Text::of("stick")),
        &cx,
    ));
    let fx = seen(update(&mut state, Msg::Press(Press::Hold), &cx));
    assert!(fx.changes.is_empty());
    assert_eq!(
        state.take_finished(),
        Some(Finished::ToVisit {
            name: Text::of("seed.txt")
        })
    );

    let _ = seen(update(&mut state, Msg::Offer(secret()), &cx));
    let fx = seen(update(&mut state, Msg::Press(Press::Vault), &cx));
    assert!(fx.changes.is_empty() && state.out().is_some());
    assert_eq!(state.answer(), Some(Answer::NoVaultOpen));
}

/// Invariant: A message that carries a secret is withheld from the log, and the sheet's state never prints the secret.
/// Regression: The offered secret's bytes reach a log line through a message's Debug.
/// Owner: app
#[test]
fn the_offered_secret_is_withheld_from_the_log() {
    let msg = Msg::Offer(secret());
    assert!(msg.public().is_none());
    assert!(!format!("{msg:?}").contains("abandon"));
}

/// Invariant: A refused write to the Outbox keeps the secret on the sheet, says the Outbox refused it, and never finishes the sheet as saved, however many files were asked for.
/// Regression: A refused PutOutbox is dropped after the secret left the sheet and the owner said the file waits in the Outbox, so the secret is gone and nothing was written.
/// Owner: app
#[test]
fn a_refused_outbox_write_keeps_the_secret_and_says_so() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = State::new();
    let mut out = secret();
    out.also = vec![(Text::of("seed-2.txt"), Bytes::new(b"two".to_vec()))];
    let _ = seen(update(&mut state, Msg::Offer(out), &cx));
    // The two writes the hold sent are waiting for their answers (the kernel's changes are not
    // served yet, so the hold's own effects cannot be run here).
    state.pending = Some(crate::state::Pending {
        names: vec![Text::of("seed.txt"), Text::of("seed-2.txt")],
        waiting: 2,
        refused: false,
        old: Vec::new(),
    });
    let refusal = || Msg::Outbox(Err(faraday_keys::Refusal::Unconfirmed));
    let fx = seen(update(&mut state, refusal(), &cx));
    assert!(fx.changes.is_empty() && state.take_finished().is_none());
    let fx = seen(update(&mut state, refusal(), &cx));
    assert!(fx.changes.is_empty());
    assert!(state.out().is_some());
    assert_eq!(state.answer(), Some(Answer::OutboxRefused));
    assert!(state.take_finished().is_none());
}

/// Invariant: A key's vault record is marked written on an online computer when the key was made online or the app runs online, and carries its pending backup mark while the key is held.
/// Regression: A seed picked into the vault in Faraday Demo loses the mark that it was handled on an online computer, because only the key's own fact was read.
/// Owner: app
#[test]
fn a_key_record_is_marked_online_when_the_app_runs_online() {
    use faraday_vault::records::{BACKUP_PENDING, MADE_ONLINE};
    assert_eq!(crate::update::key_flags(false, true), Some(MADE_ONLINE));
    assert_eq!(
        crate::update::key_flags(true, true),
        Some(BACKUP_PENDING | MADE_ONLINE)
    );
    assert_eq!(crate::update::key_flags(true, false), Some(BACKUP_PENDING));
    assert_eq!(crate::update::key_flags(false, false), None);
    // The flag the part reads is the app's mode as well as the key's own fact.
    assert!(TestView::demo().ctx().view.mode.online);
    assert!(!TestView::device().ctx().view.mode.online);
}
